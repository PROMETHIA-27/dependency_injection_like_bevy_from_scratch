#![allow(unused)]
use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

struct FunctionSystem<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

trait System {
    fn run(&mut self, resources: &mut HashMap<TypeId, RefCell<Box<dyn Any>>>);
}

trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

type StoredSystem = Box<dyn System>;

#[derive(Default)]
struct Scheduler {
    systems: Vec<StoredSystem>,
    resources: HashMap<TypeId, RefCell<Box<dyn Any>>>,
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
        self.resources
            .insert(TypeId::of::<R>(), RefCell::new(Box::new(res)));
    }
}

trait SystemParam {
    type Item<'new>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r>;
}

struct Res<'a, T: 'static> {
    value: Ref<'a, Box<dyn Any>>,
    _marker: PhantomData<&'a T>,
}

impl<T: 'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value.downcast_ref().unwrap()
    }
}

impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        Res {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow(),
            _marker: PhantomData,
        }
    }
}

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

impl<'res, T: 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    fn retrieve<'r>(resources: &'r HashMap<TypeId, RefCell<Box<dyn Any>>>) -> Self::Item<'r> {
        ResMut {
            value: resources.get(&TypeId::of::<T>()).unwrap().borrow_mut(),
            _marker: PhantomData,
        }
    }
}

macro_rules! impl_system {
    ($($params:ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<F, $($params : SystemParam + 'static),*> System for FunctionSystem<($($params ,)*), F> 
            where
                // for any two arbitrary lifetimes, a mutable reference to F with lifetime 'a
                // implements FnMut taking parameters of lifetime 'b
                for<'a, 'b> &'a mut F: 
                    FnMut($($params),*) +
                    FnMut($(<$params as SystemParam>::Item<'b>),*)
        {
            fn run(&mut self, resources: &mut HashMap<TypeId, RefCell<Box<dyn Any>>>) {
                // necessary to tell rust exactly which impl to call; it gets a bit confused otherwise
                fn call_inner<$($params),*>(
                    mut f: impl FnMut($($params),*),
                    $(
                        $params: $params
                    ),*
                ) {
                    f($($params),*)
                }

                $(
                    let $params = $params::retrieve(resources);
                )*

                call_inner(&mut self.f, $($params),*)
            }
        }
    };
}

macro_rules! impl_into_system {
    (
        $($params:ident),*
    ) => {
        impl<F, $($params: SystemParam + 'static),*> IntoSystem<($($params,)*)> for F
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
    
    ($target:ident, 4) => {
        $target!(T1, T2, T3);
        call_n_times!($target, 3);
    };
    
    ($target:ident, 5) => {
        $target!(T1, T2, T3, T4);
        call_n_times!($target, 4);
    };
    
    ($target:ident, 6) => {
        $target!(T1, T2, T3, T4, T5);
        call_n_times!($target, 5);
    };
    
    ($target:ident, 7) => {
        $target!(T1, T2, T3, T4, T5, T6);
        call_n_times!($target, 6);
    };
    
    ($target:ident, 8) => {
        $target!(T1, T2, T3, T4, T5, T6, T7);
        call_n_times!($target, 7);
    };
    
    ($target:ident, 9) => {
        $target!(T1, T2, T3, T4, T5, T6, T7, T8);
        call_n_times!($target, 8);
    };
    
    ($target:ident, 10) => {
        $target!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        call_n_times!($target, 9);
    };
}

call_n_times!(impl_system, 10);
call_n_times!(impl_into_system, 10);

mod systems;

fn main() {
    let mut scheduler = Scheduler::default();
    scheduler.add_resource(12i32);
    scheduler.add_resource(24usize);
    scheduler.add_resource("Foo");

    #[cfg(not(feature = "call"))]
    systems::add_systems(&mut scheduler);
    #[cfg(feature = "call")]
    systems::call_systems(&mut scheduler);

    scheduler.run();
}