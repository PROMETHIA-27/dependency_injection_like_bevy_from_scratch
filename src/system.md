# Defining a system

We need to define what a `System` is in our context. 

From a design perspective, we already know we can't store borrowing types; so those aren't allowed to be parameters to systems. We can also just say we'll `panic!` if a system asks for a resource we don't actually have one of. Finally we don't have anything to do with return values, so we'll prohibit them. That makes the definition of a system pretty straightforward: any function that takes `'static` parameters and returns `()`. Let's translate that to rust:
```rust
trait System<Input> {}

impl<F: FnMut()> System<()> for F {}

impl<F: FnMut(T1), T1: 'static> System<(T1,)> for F {}

// impl<F: ugh nevermind I'm bored
```
(We have to include the inputs as a type parameter on `System` for complicated type system reasons that we'll get back to later...)

Ok, so minor issue: rust doesn't have `variadic generics` like the... what, two other languages that have ever existed that do? Let's write a quick decl macro to simplify this.

(I'm sorry, I know it's unreadable and downright arcane to the uninitiated, but this is pretty much the de facto rust solution to variadic generics right now)
```rust
# trait System<Input> {}
macro_rules! impl_system {
    (
        $( 
            $($params:ident),+
        )?
    ) => {
        impl<
            F: FnMut(
                $( $($params),+ )?
            ) 
            $(, $($params: 'static),+ )?
        > 
        System<( 
            $( $($params,)+ )? 
        )> for F {}
    }
}

impl_system!();
impl_system!(T1);
impl_system!(T1, T2);
impl_system!(T1, T2, T3);
impl_system!(T1, T2, T3, T4);
```
We'll stop there for simplicity, but now it's a *lot* less typing to add more max params.
We'll have a trick later to have unlimited params, but we'll still want to impl a good amount for syntax reasons.

Ok, cool, but this is useless. How can we have one function signature that can call any of these systems?
We need to expose some way to *flatten* our input, give every system one parameter that can satisfy all of their requirements. How can we do that...?

How about this?
```rust
# use std::collections::HashMap;
# use std::any::{Any, TypeId};
trait System<Input> {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
}
```
Then this run function just needs to pull the resources out and we can wrap the actual call behind it!

Some (hacky macro) boilerplate later:
```rust
macro_rules! impl_system {
    (
        $( 
            $($params:ident),+
        )?
    ) => {
        impl<
            F: FnMut(
                $( $($params),+ )?
            ) 
            $(, $($params: 'static),+ )?
        > 
        System<( 
            $( $($params,)+ )? 
        )> for F {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $($(
                    let $params = resources.remove(&TypeId::of::<$params>()).unwrap();
                )+)?

                (self)(
                    $($(params),+)?
                );
            }
        }
    }
}
```

> Spicy sidenote here: this does permanently remove the resources from the resource store on call. We'll get back to that later, just use the scheduler no more than once for now, or refill the resources after each run.

Ok, so we've implemented a trait so that we can call some functions without *actually* knowing their params. Mostly. The trait is still parameterized with that associated type, so we can't just `Box<dyn System>`. Let's make a type erased wrapper:
```rust,ignore
#use std::collections::HashMap;
#use std::any::{Any, TypeId};
#trait System<Input> {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
#}
trait ErasedSystem {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
}

impl<S: System<I>, I> ErasedSystem for S {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
        <Self as System<I>>::run(self);
    }
}
```
Oops, that complicated type system stuff is back:
> error[E0207]: the type parameter `I` is not constrained by the impl trait, self type, or predicates

I'll save you the trouble of trying to figure out what this *really* means: Any given type can implement multiple traits. `FnMut(T)` and `FnMut(T, U)` are different traits. Therefore a type can have multiple function implementations, and we're not explicitly selecting one. Now, we don't have any fancy future type system stuff like specialization (which might not help this situation I'm not sure), but we do have structs. While `F` can implement multiple `FnMut` traits, if we wrap `F` in a struct then that struct can "select" a specific implementation; the implementation is whichever matches the struct's generic parameters, which only one implementation can do. We'll call the struct `FunctionSystem`:
```rust
#use std::marker::PhantomData;
struct FunctionSystem<Input, F> {
    f: F,
    // we need a marker because otherwise we're not using `Input`.
    // fn() -> Input is chosen because just using Input would not be `Send` + `Sync`,
    // but the fnptr is always `Send` + `Sync`.
    marker: PhantomData<fn() -> Input>,
}
```

Now let's move `System` from being on the function itself to `FunctionSystem`:
```rust
#use std::collections::HashMap;
#use std::any::{Any, TypeId};
#use std::marker::PhantomData;
#struct FunctionSystem<Input, F> {
#    f: F,
#    marker: PhantomData<fn() -> Input>,
#}
trait System {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
}

macro_rules! impl_system {
    (
        $( 
            $($params:ident),+
        )?
    ) => {
        impl<
            F: FnMut(
                $( $($params),+ )?
            ) 
            $(, $($params: 'static),+ )?
        > System for FunctionSystem<($( $($params,)+ )?), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $($(
                    let $params = *resources.remove(&TypeId::of::<$params>()).unwrap().downcast::<$params>().unwrap();
                )+)?

                (self.f)(
                    $($($params),+)?
                );
            }
        }
    }
}
```

Now that `System` takes no associated types or generic parameters, we can box it easily:
```rust
#trait System {}
type StoredSystem = Box<dyn System>;
```

We'll also want to be able to convert `FnMut(...)` to a system easily instead of manually wrapping:
```rust
#use std::marker::PhantomData;
#trait System {}
#struct FunctionSystem<Input, F> {
#    f: F,
#    marker: PhantomData<fn() -> Input>,
#}
#impl<I, F> System for FunctionSystem<I, F> {}
trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

// Example output:
// impl<F: FnMut(T1), T1> IntoSystem<(T1,)> for F {
//     type System = FunctionSystem<(T1,), Self>;

//     fn into_system(self) -> Self::System {
//         FunctionSystem {
//             f: self,
//             marker: Default::default(),
//         }
//     }
// }

macro_rules! impl_into_system {
    (
        $($(
                $params:ident
        ),+)?
    ) => {
        impl<F: FnMut($($($params),+)?) $(, $($params: 'static),+ )?> IntoSystem<( $($($params,)+)? )> for F {
            type System = FunctionSystem<( $($($params,)+)? ), Self>;

            fn into_system(self) -> Self::System {
                FunctionSystem {
                    f: self,
                    marker: Default::default(),
                }
            }
        }
    }
}

impl_into_system!();
impl_into_system!(T1);
impl_into_system!(T1, T2);
impl_into_system!(T1, T2, T3);
impl_into_system!(T1, T2, T3, T4);
```

Some helpers on `Scheduler`:
```rust
#use std::any::{Any, TypeId};
#use std::collections::HashMap;
#trait System {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
#}
#type StoredSystem = Box<dyn System>;
#struct Scheduler {
#    systems: Vec<StoredSystem>,
#    resources: HashMap<TypeId, Box<dyn Any>>,
#}
#trait IntoSystem<Input> {
#    type System: System;
#
#    fn into_system(self) -> Self::System;
#}
impl Scheduler {
    pub fn run(&mut self) {
        for system in self.systems.iter_mut() {
            system.run(&mut self.resources);
        }
    }

    pub fn add_system<I, S: System + 'static>(&mut self, system: impl IntoSystem<I, System = S>) {
        self.systems.push(Box::new(system.into_system()));
    }

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(res));
    }
}
```

All together now!
```rust
#use std::collections::HashMap;
#use std::any::{Any, TypeId};
#use std::marker::PhantomData;
struct FunctionSystem<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

trait System {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
}

macro_rules! impl_system {
    (
        $( 
            $($params:ident),+
        )?
    ) => {
        impl<
            F: FnMut(
                $( $($params),+ )?
            ) 
            $(, $($params: 'static),+ )?
        > System for FunctionSystem<($( $($params,)+ )?), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $($(
                    let $params = *resources.remove(&TypeId::of::<$params>()).unwrap().downcast::<$params>().unwrap();
                )+)?

                (self.f)(
                    $($($params),+)?
                );
            }
        }
    }
}

impl_system!();
impl_system!(T1);
impl_system!(T1, T2);
impl_system!(T1, T2, T3);
impl_system!(T1, T2, T3, T4);

trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

macro_rules! impl_into_system {
    (
        $($(
                $params:ident
        ),+)?
    ) => {
        impl<F: FnMut($($($params),+)?) $(, $($params: 'static),+ )?> IntoSystem<( $($($params,)+)? )> for F {
            type System = FunctionSystem<( $($($params,)+)? ), Self>;

            fn into_system(self) -> Self::System {
                FunctionSystem {
                    f: self,
                    marker: Default::default(),
                }
            }
        }
    }
}

impl_into_system!();
impl_into_system!(T1);
impl_into_system!(T1, T2);
impl_into_system!(T1, T2, T3);
impl_into_system!(T1, T2, T3, T4);

type StoredSystem = Box<dyn System>;

struct Scheduler {
    systems: Vec<StoredSystem>,
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl Scheduler {
    pub fn run(&mut self) {
        for system in self.systems.iter_mut() {
            system.run(&mut self.resources);
        }
    }

    pub fn add_system<I, S: System + 'static>(&mut self, system: impl IntoSystem<I, System = S>) {
        self.systems.push(Box::new(system.into_system()));
    }

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(res));
    }
}

fn main() {
    let mut scheduler = Scheduler {
        systems: vec![],
        resources: HashMap::default(),
    };

    scheduler.add_system(foo);
    scheduler.add_resource(12i32);

    scheduler.run();
}

fn foo(int: i32) {
    println!("int! {int}");
}
```

It prints `int! 12` like we want! And the user would never actually see their function get called. Mission success?

Yes, but there's obviously some rough edges. It permanently removes resources from the store each run, we have a max limit on parameters, etc, etc. We can do better, and I'll come back to this later to add some more.