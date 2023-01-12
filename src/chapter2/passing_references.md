# Passing references

> Yes, but there's obviously some rough edges. It permanently removes resources from the store each run, we have a max limit on parameters, etc, etc. We can do better, and I'll come back to this later to add some more.

Having gotten the basic architecture working, it's time to make some refinements. In this chapter we'll be focusing on two issues: The maximum limit on system parameters, and the fact that it "self destructs" every run by consuming resources. The latter will enable the former, so we'll start with allowing borrows.

First let's switch from owned values to borrowed ones, and see what we can do from there:

```rust
macro_rules! impl_system {
    (
        $(
            $($params:ident),+
        )?
    ) => {
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<
            F: FnMut(
                $( $(& $params),+ )?
            )
            $(, $($params: 'static),+ )?
        > System for FunctionSystem<($( $($params,)+ )?), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $($(
                    let $params = resources.get(&TypeId::of::<$params>()).unwrap().downcast_ref::<$params>().unwrap();
                )+)?

                (self.f)(
                    $($($params),+)?
                );
            }
        }
    }
}

macro_rules! impl_into_system {
    (
        $($(
                $params:ident
        ),+)?
    ) => {
        impl<F: FnMut($($(& $params),+)?) $(, $($params: 'static),+ )?> IntoSystem<( $($($params,)+)? )> for F {
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
{{#include src/references01.rs}}

fn foo(int: &i32) {
    println!("int! {int}");
}
```

This works, but there's a pretty obvious problem:
```rust
{{#include src/references02.rs}}
fn foo(int: i32) {
    println!("int! {int}");
}
```
> error[E0277]: the trait bound `fn(i32) {foo}: IntoSystem<_>` is not satisfied

That's not great. It'd be nice to be able to still consume resources if desired- or more likely, use mutable references. We could change it to mutable references, but then we can't use immutable references. And trying to manually implement all three would be a bit of a combinatorial explosion- every permutation of owned/&/&mut leads to something like 3^8 implementations *for the 8 parameter version alone*. Not exactly reasonable, even with macros.

Let's try something else; let's *abstract* over all possible system parameters.

```rust,noplayground
trait SystemParam {
    fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self;
}

struct Res<'a, T: 'static> {
    value: &'a T,
}

impl<'a, T: 'static> SystemParam for Res<'a, T> {
    fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self {
        let value = resources.get(&TypeId::of::<T>()).unwrap().downcast_ref::<T>().unwrap();
        Res { value }
    }
}

struct ResMut<'a, T: 'static> {
    value: &'a mut T,
}

impl<'a, T: 'static> SystemParam for ResMut<'a, T> {
    fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self {
        let value = resources.get_mut(&TypeId::of::<T>()).unwrap().downcast_mut::<T>().unwrap();
        ResMut { value }
    }
}

struct ResOwned<T: 'static> {
    value: T
}

impl<T: 'static> SystemParam for ResOwned<T> {
    fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self {
        let value = *resources.remove(&TypeId::of::<T>()).unwrap().downcast::<T>().unwrap();
        ResOwned { value }
    }
}
```

`SystemParam` provides the `retrieve` function which is where our logic for gethering resources lives. Conveniently, this simplifies the (now once again outdated) macro implementation.
Res/ResMut/ResOwned map to &/&mut/owned respectively. They also closely resemble some of bevy's own `SystemParam`s. 

Great, now let's try to compile and-
> error: lifetime may not live long enough

~~oh wow lifetime errors my favorite~~

This seems like an easy fix at first...
```rust,noplayground,ignore
// The modification is the same for ResMut/Owned
impl<'a, T: 'static> SystemParam for Res<'a, T> {
    fn retrieve(resources: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Self {
        let value = resources.get(&TypeId::of::<T>()).unwrap().downcast_ref::<T>().unwrap();
        Res { value }
    }
}
```
But this changes the function signature, so we need a lifetime in `SystemParam`
```rust,noplayground,ignore
trait SystemParam<'a> {
    fn retrieve(resources: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Self;
}
```
(which of course infects the macros AGAIN (hence why I haven't updated them yet))

This leads to yet another lifetime error in implementing systems, as they try to pass in a `&'_ mut HashMap...` rather than `&'a mut HashMap...`.
```rust,noplayground,ignore
trait System<'a> {
    fn run(&mut self, resources: &'a mut HashMap<TypeId, Box<dyn Any>>);
}
```

Which then impacts `IntoSystem`...
```rust,noplayground,ignore
trait IntoSystem<'a, Input> {
    type System: System<'a>;

    fn into_system(self) -> Self::System;
}
```

AND `StoredSystem`...
```rust,noplayground,ignore
type StoredSystem = Box<dyn for<'a> System<'a>>;
```

And finally `add_system`
```rust
{{#include src/references03.rs:before}}
pub fn add_system<I, S: for<'a> System<'a> + 'static>(&mut self, system: impl for<'a> IntoSystem<'a, I, System = S>) {
    self.systems.push(Box::new(system.into_system()));
}
{{#include src/references03.rs:after}}
```

WHEW! Glad that's over. Now it's time for a *real error*. One that isn't just us needing to slap annotations everywhere.
> error[E0499]: cannot borrow `*resources` as mutable more than once at a time

Yep! We're mutably borrowing resources multiple times for variants with > 1 parameter.
How do we solve this, using all the clever tools rust provides to create a safe, powerful solution-
```rust
{{#include src/references04.rs}}
trait SystemParam<'a> {
    fn retrieve(resources: &'a HashMap<TypeId, Box<dyn Any>>) -> Self;
}

impl<'a, T: 'static> SystemParam<'a> for Res<'a, T> {
    fn retrieve(resources: &'a HashMap<TypeId, Box<dyn Any>>) -> Self {
        let value = resources.get(&TypeId::of::<T>()).unwrap().downcast_ref::<T>().unwrap();
        Res { value }
    }
}

// struct ResMut<'a, T: 'static> {
//     value: &'a mut T,
// }

// impl<'a, T: 'static> SystemParam for ResMut<'a, T> {
//     fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self {
//         let value = resources.get_mut(&TypeId::of::<T>()).unwrap().downcast_mut::<T>().unwrap();
//         ResMut { value }
//     }
// }

// struct ResOwned<T: 'static> {
//     value: T
// }

// impl<T: 'static> SystemParam for ResOwned<T> {
//     fn retrieve(resources: &mut HashMap<TypeId, Box<dyn Any>>) -> Self {
//         let value = *resources.remove(&TypeId::of::<T>()).unwrap().downcast::<T>().unwrap();
//         ResOwned { value }
//     }
// }
```

We'll burn that bridge when we get to it, I don't have the time for interior mutability or unsafe shenanigans right now.
Because unfortunately that lifetime stuff is back.
> error: implementation of `System` is not general enough

We can't actually pass any existing system to `add_system`, because it requires that the system implement both `System` and `IntoSystem` for *all* lifetimes.
(That's what that `for<'a>` bit means). It doesn't, it's only implemented for the lifetime of its parameter, so that won't work. And if that won't work, then we can't box it like this either, so it looks like we'll need to go back to the drawing board. Why not take a look at how bevy approaches this?
```rust,ignore,noplayground
impl<Out, Func: Send + Sync + 'static, $($param: SystemParam),*> SystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($param),*) -> Out +
                FnMut($(SystemParamItem<$param>),*) -> Out, Out: 'static
```
How interesting... and what is `SystemParamItem`?
```rust,ignore,noplayground
/// Shorthand way of accessing the associated type [`SystemParam::Item`] for a given [`SystemParam`].
pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;
```
Ah, "easy". So `SystemParam` has a GAT called `Item` which is the same as the `SystemParam`, but with a new lifetime. They can take the function with some irrelevant lifetime, and then give it a new lifetime of the passed in resources. And while the type alias makes it shorter, I'm going to go without it to illustrate what it really means. Very complicated, and very clever. Let's do it!
```rust
trait SystemParam {
    type Item<'new>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, Box<dyn Any>>) -> Self::Item<'r>;
}

impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, Box<dyn Any>>) -> Self::Item<'r> {
        Res { value: resources.get(&TypeId::of::<T>()).unwrap().downcast_ref().unwrap() }
    }
}

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F, $($params: SystemParam),*> System for FunctionSystem<($($params,)*), F> 
            where
                for<'a, 'b> &'a mut F: 
                    FnMut( $($params),* ) + 
                    FnMut( $(<$params as SystemParam>::Item<'b>),* )
        {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                fn call_inner<$($params),*>(
                    mut f: impl FnMut($($params),*),
                    $($params: $params),*
                ) {
                    f($($params),*)
                }

                $(
                    let $params = $params::retrieve(resources);
                )*

                call_inner(&mut self.f, $($params),*)
            }
        }
    }
}

macro_rules! impl_into_system {
    (
        $($params:ident),*
    ) => {
        impl<F, $($params: SystemParam),*> IntoSystem<($($params,)*)> for F 
            where
                for<'a, 'b> &'a mut F: 
                    FnMut( $($params),* ) + 
                    FnMut( $(<$params as SystemParam>::Item<'b>),* )
        {
            type System = FunctionSystem<($($params,)*), Self>;

            fn into_system(self) -> Self::System {
                FunctionSystem {
                    f: self,
                    marker: Default::default(),
                }
            }
        }
    }
}
{{#include src/references05.rs}}
```
(The `call_inner` bit is necessary to tell rust which function impl to call, it gets a bit confused otherwise.)

And this works! Perfectly! No weird errors, we've finally solved the ability to pass &/&mut/own... right, we put that off for a bit. But we have the infrastructure!

And this infrastructure lends itself perfectly to allowing unlimited parameters, which we'll do next.
