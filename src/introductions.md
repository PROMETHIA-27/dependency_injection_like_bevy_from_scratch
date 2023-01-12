# Introductions

## What is this book about?

Several rust libraries use a pattern something like this (in this example, bevy):
```rust,ignore
fn main() {
    App::new()
        .add_system(foo)
        .add_system(bar)
        .run();
}

fn foo(query: Query<Foo, With<Bar>>) {
    // some code
}

fn bar(query: Query<Bar, Without<Foo>>) {
    // some other code
}
```
Most users can intuitively grasp that this causes the app to automatically call the systems `foo` and `bar` 
once per frame, but very few are able to easily figure out *how* this is possible. This book aims to explain
how this works, starting from scratch.

## What is Dependency Injection?

Dependency injection is a needlessly complicated way to phrase "asking for things instead of providing them". 

An easy example of dependency injection would be `Iterator::map()`; you provide a function/closure which asks for
an item to map and maps it, and the iterator itself "injects" that "dependency". 

In this case we're mimicking what
Bevy Engine does. An `App` is created, provided with `System`'s, and those `System`'s are called automatically. `System`'s have various parameters which are automatically known and provided by the `App`. The parameters are the dependencies, and the app is "injecting" them.
Dependency injection is a useful pattern, and most people have probably used it at least *somewhere* even if they don't know it by name.

## Is this book exclusively about rust and bevy?

About rust: Yes, but the techniques within can be applied to other languages *if* those languages have the features to support it. 

About bevy: Sort of. This book is heavily inspired by it, but these techniques can certainly be applied to other rust projects, and have already shown up before in libraries like [`axum`](https://docs.rs/axum/latest/axum/extract/index.html). And overall the technique shown will be simpler than what bevy actually does.

## How much rust do I need to know to understand this?

I'll try to aim to make this as easily understandable as possible, but understanding of traits, `dyn Trait`s, tuples, a little bit of lifetimes, and other basic rust knowledge will likely be required.

