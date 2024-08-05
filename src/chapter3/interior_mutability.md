# The easy way out

So, our goal is to get disjoint mutable access to resources in the world and provide them to 
systems **that are run strictly serially (singlethreaded) (!!!)**. This isn't easy, because the borrow
checker has a hard time when you try to get multiple mutable references inside a data structure
(such as our hashmap) at the same time.

Luckily for us, rust provides some escape hatches. First I'll cover the safe, easier way, then 
*the fun way*. 

The primary tool we're going to use for the "easy way" is a concept called

# Interior Mutability:tm:

Very scary sounding, but actually very simple. Interior mutability simply means there exists a function
for this type roughly like `fn(&self) -> &mut Self::Inner`. This, of course, *seems* to violate a 
fundamental rule of the borrow checker, that you cannot mutate from an immutable reference. 

But sometimes you *need* to do that, so rust provides the [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html)
type. Then, safe types were built on top of it for our convenience. In this case, we're going
to use a neat type called [`RefCell`](https://doc.rust-lang.org/std/cell/struct.RefCell.html).

`RefCell` is a type that allows safe interior mutability by checking at *runtime* if it's being accessed
correctly. Pretend it looks like this on the inside (this is pseudocode which will not compile for the sake of
being clearer to read, nor does it exactly resemble RefCell's actual implementation which is somewhat
more optimized):
```rust,ignore
enum Borrow {
    None,
    Immutable(NonZeroUsize),
    Mutable(NonZeroUsize),
}

struct RefCell<T> {
    cell: UnsafeCell<T>,
    borrows: Borrow,
}
```

Then when you attempt to borrow it:
```rust,ignore
// immutable
match &mut self.borrows {
    Borrow::None => {
        self.borrows = Borrow::Immutable(1);
        unsafe { &*self.cell.get() }
    }
    Borrow::Immutable(x) => {
        *x += 1;
        unsafe { &*self.cell.get() }
    },
    Borrow::Mutable(_) => panic!(),
}

// mutable
match &mut self.borrows {
    Borrow::None => {
        self.borrows = Borrow::Mutable(1);
        unsafe { &mut *self.cell.get() }
    }
    Borrow::Immutable(_) => panic!(),
    Borrow::Mutable(_) => panic!(),
}
```

It increments counters whenever you borrow, or panics if you attempt to make invalid 
borrows like an immutable reference when a mutable reference exists.

Then, instead of returning `&T` or `&mut T`, it returns the special types 
[`Ref`](https://doc.rust-lang.org/std/cell/struct.Ref.html) and 
[`RefMut`](https://doc.rust-lang.org/std/cell/struct.RefMut.html), which are `Deref<Target = T>`.
When these are dropped, they decrement the borrow counter.

It's like a runtime borrow checker! This is super useful but very critically: *not* threadsafe,
as I alluded to at the top with heavy emphasis. A threadsafe alternative would be something like
[`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html) or 
[`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html), which have different semantics.
RwLock is a closer match but, to my knowledge, rarely actually useful compared to a Mutex for complicated
performance reasons.

But we're not threading, so let's just use `RefCell`. 

First, an observation:
```rust,compile_fail
let v = vec![1, 2];
let x = &mut v[0];
let y = &mut v[1];

println!("{x} {y}");
```

This doesn't compile, since we're violating the principle of mutability XOR aliasing.
But *this* works:

```rust
#use std::cell::RefCell;
let v = vec![
    RefCell::new(1), 
    RefCell::new(2),
];
let mut x = v[0].borrow_mut();
let mut y = v[1].borrow_mut();

*x += 1;
*y += 1;

println!("{x} {y}");
```

This should make at least some intuitive sense now, but to be more clear:
1. `.borrow_mut()` takes `&self`, not `&mut self`
2. Thus `x` and `y` are *immutably* borrowing from `v` 
3. `x` and `y` are of type [`RefMut`](https://doc.rust-lang.org/std/cell/struct.RefMut.html), which
can provide a mutable reference to its inner type (but they must be marked `mut` to get a mutable
reference to them to do so)
4. `RefMut` impls `Display for T: Display` and `Deref<Target = T>`, so we can basically use them
as if they're `&mut T`

Now we should understand the tool well enough to put it to use.

## Implementation

First let's redefine a few things:
- Add `RefCell` into `Schedule`'s `resources` (and also add default derive for convenience)
```rust,ignore
#[derive(Default)]
struct Scheduler {
    systems: Vec<StoredSystem>,
    resources: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}
```
- Wrap resources in `RefCell` in `add_resource`
```rust,ignore
impl Scheduler {
    // ...

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        self.resources
            .insert(TypeId::of::<R>(), RefCell::new(Box::new(res)));
    }
}
```
- Add `RefCell` to signature here
```rust,ignore
trait System {
    fn run(&mut self, resources: &mut HashMap<TypeId, RefCell<Box<dyn Any>>>);
}
```
- And here
```rust,ignore
trait SystemParam {
    type Item<'new>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r>;
}
```
- Res needs to store a `Ref<Box<dyn Any>>` now instead of `&T`, or the `Ref` will be dropped 
early
```rust,ignore
struct Res<'a, T: 'static> {
    value: Ref<'a, Box<dyn Any>>,
    _marker: PhantomData<&'a T>,
}
```
- Add [`Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html) impl to `Res` for convenience
```rust,ignore
impl<T: 'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.downcast_ref().unwrap()
    }
}
```
- Add a `.borrow()` here to implement `Res` trivially
```rust,ignore
impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        Res {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow(),
            _marker: PhantomData,
        }
    }
}
```

And this gives us a functioning `Res` again! Now let's implement `ResMut`:

- Define `ResMut` (plus `Deref` impls):
```rust,ignore
struct ResMut<'a, T: 'static> {
    value: RefMut<'a, Box<dyn Any>>,
    _marker: PhantomData<&'a mut T>,
}

impl<T: 'static> Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.downcast_ref().unwrap()
    }
}

impl<T: 'static> DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value.downcast_mut().unwrap()
    }
}
```
- Impl `SystemParam` for it:
```rust,ignore
impl<'res, T: 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        ResMut {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow_mut(),
            _marker: PhantomData,
        }
    }
}
```

And there we go! We can now access multiple resources mutably from systems.

However, we can't actually add owned resources still- one might notice that bevy does not have anything
like this anyway. If you wanted to accomplish this, one way would be to wrap the `RefCell` in 
`resources` with `Option`, and then you can use `.take()` to remove a resource from `resources` 
entirely to define `SystemParam::retrieve` for the owned resource. However this would be niche and 
error prone to use, so I'm not going to do it myself.

## Final Product
```rust
{{#include src/interior_mutability.rs:All}}
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

Pretty cool! But this does have one sharp edge (if you run this, it will panic):
```rust,should_panic
{{#include src/interior_mutability.rs:All}}
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

We of course still can't borrow *the same resource* mutably multiple times at once, and `RefCell`
will prevent this by panicking if we ever try to construct an ill-formed system like this. Bevy will
do something similar, but with a better error message; We will show how in the next section.