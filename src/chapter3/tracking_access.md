# Tracking accesses

Tracking accesses is conceptually fairly simple for our typemap store. We just need to create a 
set of `TypeId` -> `Access` mappings and check that they don't conflict. 

First, we define `Access` as being either `Read` (shared, immutable) or `Write` (exclusive, mutable):
```rust,ignore
{{#include src/tracking_access.rs:Access}}
```

Define a new map type to use to track accesses, for convenience:
```rust,ignore
{{#include src/tracking_access.rs:AccessMap}}
```

Alter the signature of `System::run()` to have the `AccessMap`:
```rust,ignore
{{#include src/tracking_access.rs:System}}
```

Add a new method to `SystemParam`, so now parameters can declare their accesses (and report conflicts)
```rust,ignore
{{#include src/tracking_access.rs:SystemParam}}
```

Implement it as appropriate:
```rust,ignore
{{#include src/tracking_access.rs:ResSystemParam}}

{{#include src/tracking_access.rs:ResMutSystemParam}}
```

Now we can call it before we call `retrieve()` and use it as a justification that our accesses are
not invalid:
```rust,ignore
{{#include src/tracking_access.rs:impl_system_macro}}
```

tweak the scheduler to track accesses:
```rust,ignore
{{#include src/tracking_access.rs:Scheduler}}

{{#include src/tracking_access.rs:SchedulerImpl}}
```

And now we catch that erroneous case!
```rust
{{#rustdoc_include src/tracking_access.rs:0:0}}
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

With the conclusion of this chapter, I'm approaching the end of the list of topics I can/want to
touch on via this dependency injection framework trick. If you have ideas for topics you'd like
to see covered, relevant to dependency injection or not, let me know in the github
issues for this mdbook!

The next chapter will cover eliminating the unnecessary `TypeId` comparison we make after extracting values
from `TypeMap`, by making a custom collection using unsafe.