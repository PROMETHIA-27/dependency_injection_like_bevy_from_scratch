# Macros

This section will serve as an iterative introduction to rust declarative macros. Rust has 2 kinds
of macro, declarative and procedural. Declarative macros use a strange syntax involving pattern matching,
whereas a procedural macro is (mostly) normal rust code that manipulates an input syntax tree. 

I will not be covering procedural macros here, as they're generally for much more "polished" approaches
like `bevy_reflect`, `thiserror`, and other derive or attribute macros. Procedural macros are rarely
used in function style like `my_macro!()`, whereas declarative macros can only be put in function position.

First, a use case: Why do we want to use a declarative macro? In this case, let's look at the `System` trait
implementations:
```rust,ignore
impl<F: FnMut()> System for FunctionSystem<(), F> {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
        (self.f)()
    }
}

impl<F: FnMut(T1), T1: 'static> System for FunctionSystem<(T1,), F> {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
        let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();

        (self.f)(_0)
    }
}

impl<F: FnMut(T1, T2), T1: 'static, T2: 'static> System for FunctionSystem<(T1, T2), F> {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
        let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
        let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();

        (self.f)(_0, _1)
    }
}
```

This is a mouthful of boilerplate, and for only 3 impls. We'd likely prefer to have 16 or 17. We can
dramatically shrink this code and the amount of effort required to add impls with a decl macro.

First, let's take a reasonably representative implementation from our target output:
```rust,ignore
impl<F: FnMut(T1, T2), T1: 'static, T2: 'static> System for FunctionSystem<(T1, T2), F> {
    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
        let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
        let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();

        (self.f)(_0, _1)
    }
}
```
(it's ideal to choose one that "scales", i.e. shows off the pattern well; usually picking 
the implementation corresponding to "2" works best. In this case, the "2" is "2 parameters".)

First, let's just wrap it in macro declaration syntax:
```rust,ignore
macro_rules! impl_system {
    () => {
        impl<F: FnMut(T1, T2), T1: 'static, T2: 'static> System for FunctionSystem<(T1, T2), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
                let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();

                (self.f)(_0, _1)
            }
        }
    };
}
```
This produces a macro that takes no parameters and will just spit out this impl verbatim.

Now we need to identify what changes are happening between implementations. We need to change:
- Instances of T1, T2, etc.
- The `_0`, `_1`, etc. chain
- The function call parameters

Crucially, these are all repetitions derived from the list of type arguments (T1, T2, ...). 
That means our parameters to the macro will be `T1, T2, ...`, and we can extract the rest from there.
```rust,ignore
macro_rules! impl_system {
    ($($params:ident),*) => {
        // ...
    };
}
```

`$` is a major symbol in decl macros, indicating some decl-macro specific syntax. `$[name]:[type]` syntax
declares a syntax variable; in this case we bind some ident (any string that would be valid for e.g. a function,
variable, or type name, among others) to the name `param`. We can access this variable inside the 
macro body with `$param`. 

Then, we wrapped that variable in `$()*` to signify that we want to match
*0 or more* of it. This means we need to begin a repetition in the macro body to actually access
the variable, since we match it 0 or more times.

Finally, we slip a `,` into `$(),*` to signify that when we match 0 or more times, there must be a
comma between each element *but not one at the end*. If we wanted to optionally consume a trailing comma,
we could add `$(,)?` after the parameter to mean "0 or 1 commas" like so: `$($params:ident),* $(,)?`.

Alternatively, if we wanted to **always** match a trailing comma, we could instead do this:
`$($params:ident ,)*` which would match "an ident followed by a comma, 0 or more times".

So, we now have some parameters available; probably a sequence of `T1, T2, ..., TN`. The first thing
we can do is replace our hardcoded type lists:
```rust,ignore
macro_rules! impl_system {
    ($($params:ident),*) => {
        impl<F: FnMut($($params),*), T1: 'static, T2: 'static> System for FunctionSystem<($($params ,)*), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
                let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();

                (self.f)(_0, _1)
            }
        }
    };
}
```

Note that the `FnMut` receives a plain list of type arguments, but the `FunctionSystem` receives a tuple; 
thus we use `$($params),*` for the FnMut but `$($params ,)*` for the tuple, to ensure the `T1` case
actually creates a valid tuple and not just a parenthesized `T1`. Technically, we could use `$($params ,)*`
for both since rust is good about respecting trailing commas, generally.

As you can see, the syntax to extract the syntax variables is very similar to the syntax to match 
them in the first place. The next thing we want to do is replace those `TN: 'static` bits; they're
slightly more complex.

```rust,ignore
macro_rules! impl_system {
    ($($params:ident),*) => {
        impl<F: FnMut($($params),*), $($params : 'static),*> System for FunctionSystem<($($params ,)*), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
                let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();

                (self.f)(_0, _1)
            }
        }
    };
}
```

We just insert new verbatim syntax into the repetition just like when we want to match a comma
after every ident. Next, we need to deal with those variables, but this presents a problem; how do
we come up with a unique variable name for each one? 

> That's the neat part, you don't

We'll just name the variable the exact same thing as its type. The compiler'll figure it
out, it's fine.

```rust,ignore
macro_rules! impl_system {
    ($($params:ident),*) => {
        impl<F: FnMut($($params),*), $($params : 'static),*> System for FunctionSystem<($($params ,)*), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $(
                    let $params = *resources.remove(&TypeId::of::<$params>()).unwrap().downcast::<$params>().unwrap();
                )*

                (self.f)(_0, _1)
            }
        }
    };
}
```

And look, now that the variables are named the same thing as their types, we can just replace the
parameters to the function call like we did in the first step.

Also, let's add some lint suppressors, because the generated code will raise some pointless warnings.

```rust,ignore
macro_rules! impl_system {
    ($($params:ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<F: FnMut($($params),*), $($params : 'static),*> System for FunctionSystem<($($params ,)*), F> {
            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
                $(
                    let $params = *resources.remove(&TypeId::of::<$params>()).unwrap().downcast::<$params>().unwrap();
                )*

                (self.f)($($params),*)
            }
        }
    };
}
```

Now just call the macro a few times:

```rust,ignore
impl_system!();
impl_system!(T1);
impl_system!(T1, T2);
impl_system!(T1, T2, T3);
impl_system!(T1, T2, T3, T4);
impl_system!(T1, T2, T3, T4, T5);
// and so on
```

And now we've massively cut down on code duplication, at the cost of code obfuscation to those
not versed in the dark art of macros.

But this is still somewhat... duplicate-y. What if we wrote another macro to invoke this macro a number of times?

If we don't need to parameterize it, we can hardcode it to expand, say, 16 times:
```rust,ignore
macro_rules! call_16_times {
    ($target:ident) => {
        $target!();
        $target!(T1);
        $target!(T1, T2);
        // etc.
    };
}
```

Or, we can even make use of pattern matching to make it inductive:
```rust,ignore
macro_rules! call_n_times {
    ($target:ident, 1) => {
        $target!();
    };

    ($target:ident, 2) => {
        $target!(T1);
        call_n_times!($target, 1);
    };
    
    ($target:ident, 3) => {
        $target!(T1, T2);
        call_n_times!($target, 2);
    };

    // etc.
}
```

At this point, we're starting to spin our wheels in a [turing tarpit](https://en.wikipedia.org/wiki/Turing_tarpit), so if you're going any further
than this consider switching to procedural macros. But to prove this all works:
```rust
{{#include src/macros.rs:All}}
call_n_times!(impl_system, 3);

fn main() {
    let mut scheduler = Scheduler {
        systems: vec![],
        resources: HashMap::default(),
    };

    scheduler.add_system(foo);
    scheduler.add_resource(12i32);
    scheduler.add_resource(24f32);

    scheduler.run();
}

fn foo(int: i32, float: f32) {
    println!("int! {int} float! {float}");
}
```