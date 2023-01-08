# Setting up the scheduler

In order to illustrate what's going on, we'll want a data structure that can store resources to be queried, our systems, and run them. We'll keep it extremely simple:
```rust
# use std::collections::HashMap;
# use std::any::{Any, TypeId};
# type StoredSystem = ();
struct Scheduler {
    systems: Vec<StoredSystem>,
    resources: HashMap<TypeId, Box<dyn Any>>,
}
```
The scheduler stores `StoredSystem`'s (don't worry, we haven't defined those yet) and uses a basic `TypeMap` which can store one item of every type (provided that the item lives for `'static` a.k.a. is not a borrow and does not include a borrow).

What about that `StoredSystem`? We'll get back to those later, for now just supply it with a dummy definition:
```rust
struct StoredSystem;
```