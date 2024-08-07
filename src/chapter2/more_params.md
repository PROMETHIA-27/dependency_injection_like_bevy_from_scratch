# More parameters

Now that we have `SystemParam` in place, it'll be easy to expand this to work with unlimited parameters. We just need one crucial idea: what if a tuple of `SystemParam` is, itself, a `SystemParam`?
Let's implement:
```rust,ignore
impl<T1: SystemParam, T2: SystemParam> SystemParam for (T1, T2) {
    type Item<'new> = (T1::Item<'new>, T2::Item<'new>);

    fn retrieve<'r>(resources: &'r HashMap<TypeId, Box<dyn Any>>) -> Self::Item<'r> {
        (
            T1::retrieve(resources),
            T2::retrieve(resources),
        )
    }
}

fn foo(int: (Res<i32>, Res<u32>)) {
    println!("int! {} uint! {}", int.0.value, int.1.value);
}
```
It just works!

Now you may wonder:
> But this is only two items? This doesn't actually give us "unlimited" parameters, just slightly more?

But this is actually sufficient to have unlimited parameters:
```rust,ignore,noplayground
fn foo(int: (Res<One>, (Res<Two>, (Res<Three>, Res<Four>)))) {
    // ...
}
```
And so on. As I hinted at in chapter 1, there's a syntax cost to this, but it's alleviated by implementing up to 16-tuples, so just implement for bigger tuples.
Maybe investigate the macros section in chapter one for a convenient way to do this!

And that's it, actually. We can nest parameters indefinitely to pass infinite parameters. Next time we'll go back to that aliasing issue and figure out how to get disjoint mutable access to resources.
