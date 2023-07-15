# The spicy way out

Now that we've seen how to accomplish what we wanted to the safe way, we're going to take a look
at the unsafe way, which is what bevy uses (although we won't use *exactly* the same techniques
as bevy).

The benefits of the unsafe method are that we can avoid a lot of redundant checks- for example,
we first look up a value in `resources` by its `TypeId`, which we know is always accurate, and then
we call `downcast_ref/mut` which does the check *again*. These extra checks are merely wasted
cycles. We want to optimize our runtime by eliminating them, which requires a bit of unsafe sometimes.

# Prelude and Disclaimers

The first thing I want to note: **unsafe is very dangerous**. 

This is not "C mode" rust, the amount of Undefined Behavior rust uses goes well beyond C; the difference is that it's well documented. 

Almost every unsafe function will (and *all* **should**) list the invariants that the caller must uphold
to soundly call the function, and every call to an unsafe function **should** be justified with 
a convincing argument for why every invariant of the function is upheld. 

One slip up can cause major problems (think "remote code execution by bad actors"), and it is **far** 
more difficult to  even find Undefined Behavior than almost any other bug category. 

With that said, we have some tools to protect us. The Miri interpreter can run *most* unsafe rust
code and verify with *reasonable* accuracy that it is safe. The issue is it works like a unit test
moreso than a type system- it can tell you if the code you *ran* did something wrong, but not if 
the code you *could* run will. 

It's also extremely important to have other people familiar with unsafe code check your code. Peer reviews
work *wonders* here. If anyone is so much as *suspicious* of your code, you need to spend more time
verifying and testing it.

I *have* tested this code and *have* run it by some experts, so the precise code here *should* be
sound. Even minor changes to the code may change that, if you're not careful. This also happens
to be the kind of code that Miri should be rather good at checking, but Miri is not guaranteed
to be clear of false positives *or* negatives. Especially because the precise rules of rust's memory
model are in flux, meaning depending on what rules you use the answer may change.

# Design

With that out of the way, let's start outlining what we need to do:
Our goal here is disjoint mutable access to elements of a hashmap. This is usually impossible because
getting mutable access to an element of the hashmap borrows the whole hashmap mutably (as applies to most
data structures in rust). 

Currently, we're accomplishing 
this with runtime refcounting, but that's not the most efficient way to do it. So, we need an implementation
that will allow us to get that disjoint access with something cheaper than refcounting.

Bevy does this by making use of `UnsafeCell`, and creating abstractions out of that to minimize the
number of invariants that need to be handled at any given time. 

In a previous iteration of this
chapter, I reached for raw pointers instead. While that gave more opportunities for learning about
how to handle unsafe code, I've decided that it actually does significantly complicate the code
and those lessons can be postponed to a future chapter. 

We'll also be using `UnsafeCell`. It has relatively few invariants to uphold:
>  Ensure that the access is unique (no active references, mutable or not) when casting to &mut T, 
and ensure that there are no mutations or mutable aliases going on when casting to &T

That's not bad. Going with `UnsafeCell` has an additional benefit; this entire process can be viewed
as a simple optimization. We're just going to "cache" the runtime refcounting! Do it all up-front
when a system is being created/initialized and then we'll know it's correct from that point on.

# Implementation

So we have an idea of what to do; let's begin:

First, I'll add a type alias for `TypeMap` because I'm tired of switching it around everywhere. Then
we wrap the values in `UnsafeCell`, so we can get a single mutable reference to each element without
mutably borrowing the entire map. Note that we *are* still immutably borrowing the entire map as long
as we're keeping track of lifetimes correctly.
```rust,ignore
{{#include src/unsafe.rs:TypeMap}}
```
We'll simplify `Res` and `ResMut`'s internals again
```rust,ignore
{{#include src/unsafe.rs:Res}}
```
```rust,ignore
{{#include src/unsafe.rs:ResMut}}
```
And set up the scheduler
```rust,ignore
{{#include src/unsafe.rs:Scheduler}}
```
```rust,ignore
{{#include src/unsafe.rs:SchedulerImpl}}
```

Here's where the unsafe begins. I'm going to mark `SystemParam::retrieve` as `unsafe`, so that it 
doesn't have to make safety checks.

This means we'll have to do them elsewhere, but we can reduce the number of times the checks need to 
happen because of it. This means that the function will be unsafe to call, and so we should give
a convincing argument when calling it that we are upholding invariants.
```rust,ignore
{{#include src/unsafe.rs:SystemParamRetrieve}}
```

This makes the implementation straightforward- we look up the value we want, then convert it to a 
reference.
```rust,ignore
{{#include src/unsafe.rs:ResSystemParam}}
```
```rust,ignore
{{#include src/unsafe.rs:ResMutSystemParam}}
```

Cool, this should, basically\*, work as is.

> \*im lying lol lmao

# Final Product

```rust
{{#include src/unsafe.rs:All}}

fn main() {
    let mut scheduler = Scheduler::default();
    scheduler.add_system(foo);
    scheduler.add_system(bar);
    scheduler.add_resource(12i32);
    scheduler.add_resource("Hello, world!");

    scheduler.run();
}

fn foo(mut int: ResMut<i32>) {
    *int += 1;
}

fn bar(statement: Res<&'static str>, num: Res<i32>) {
    #assert_eq!(*num, 13);
    println!("{} My lucky number is: {}", *statement, *num);
}
```

It prints what we want! Awesome! But we seem to be forgetting something:
```rust
{{#rustdoc_include src/unsafe.rs:0:0}}
fn main() {
    let mut scheduler = Scheduler::default();
    scheduler.add_system(spooky);
    scheduler.add_resource(13i32);

    scheduler.run();
}

fn spooky(_foo: ResMut<i32>, _bar: ResMut<i32>) {
    println!("Haha lmao");
}
```

***Oof.*** 

This hopefully makes you viscerally uncomfortable, because this is blatant Undefined Behavior. While it (probably)
doesn't manifest in anything noticeable currently, it has the potential to do anything.

Including and certainly not limited to installing a keylogger that will steal your bank credentials. 

Obviously\* it
won't, but according to the definition of Undefined Behavior, yes, the compiler is *specifically allowed* to do that.
Instead it'll likely just cause some nondeterministic bugs that will only cause problems when you're
not trying to observe them, making debugging horrific.

You might also have noticed that I failed to follow my own advice:
> ... and so we should give a convincing argument when calling it that we are upholding invariants

That's because... we don't have one! We aren't upholding invariants! Hard to prove something which 
is false.

To fix this, let's go to the next section to see how to track resource accesses.

> \*If you don't in any way accept user input, otherwise it would mean there's a non-zero chance
you opened up an arbitrary code execution exploit and now it might install a keylogger actually

> As an exercise for the reader, run the above failing example in miri (this may be easiest through
rust playground); notice that it (*hopefully*) catches this UB! And also notice that it *doesn't*
see anything wrong with the previous, working example, even though it has the potential to cause the
exact same UB!