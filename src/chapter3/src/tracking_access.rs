// ANCHOR: All
use std::any::{Any, TypeId};
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

// ANCHOR: TypeMap
type TypeMap = HashMap<TypeId, UnsafeCell<Box<dyn Any>>>;
// ANCHOR_END: TypeMap

// ANCHOR: impl_system_macro
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
            fn run(&mut self, resources: &TypeMap, accesses: &mut AccessMap) {
                fn call_inner<$($params),*>(
                    mut f: impl FnMut($($params),*),
                    $($params: $params),*
                ) {
                    f($($params),*)
                }

                $(
                    $params::accesses(accesses);
                )*

                // SAFETY:
                // Every access here is proven to be nonconflicting because of the calls above to
                // `access`.
                $(
                    let $params = unsafe { $params::retrieve(resources) };
                )*

                call_inner(&mut self.f, $($params),*)
            }
        }
    }
}
// ANCHOR_END: impl_system_macro

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

// ANCHOR: Access
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Access {
    Read,
    Write,
}
// ANCHOR_END: Access

// ANCHOR: AccessMap
type AccessMap = HashMap<TypeId, Access>;
// ANCHOR_END: AccessMap

// ANCHOR: SystemParam
trait SystemParam {
    type Item<'new>;

    /// For safety, this function must panic if there are any conflicting accesses, and it must
    /// accurately record its accesses so that a future call can panic if there are conflicting
    /// accesses.
    fn accesses(access: &mut AccessMap);

    // ANCHOR: SystemParamRetrieve
    /// SAFETY:
    /// - The caller must not have active conflicting references to resources that this function will access
    unsafe fn retrieve<'r>(resources: &'r TypeMap) -> Self::Item<'r>;
    // ANCHOR_END: SystemParamRetrieve
}
// ANCHOR_END: SystemParam

// ANCHOR: ResSystemParam
impl<'res, T: 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn accesses(access: &mut AccessMap) {
        assert_eq!(
            *access.entry(TypeId::of::<T>()).or_insert(Access::Read),
            Access::Read,
            "conflicting access in system; attempting to access {} mutably and immutably at the same
            time",
            std::any::type_name::<T>(),
        );
    }

    unsafe fn retrieve<'r>(resources: &'r TypeMap) -> Self::Item<'r> {
        let value = resources[&TypeId::of::<T>()].get();

        // SAFETY:
        // The caller asserts that there are no conflicting accesses, and the pointer is definitely
        // valid as it was obtained directly from `UnsafeCell`. Its lifetime will be constrained
        // to the lifetime of the map it was obtained from, so it cannot dangle.
        let value = unsafe { &*value };

        let value = value.downcast_ref::<T>().unwrap();

        Res { value }
    }
}
// ANCHOR_END: ResSystemParam

// ANCHOR: ResMutSystemParam
impl<'res, T: 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    fn accesses(access: &mut AccessMap) {
        match access.insert(TypeId::of::<T>(), Access::Write) {
            Some(Access::Read) => panic!(
                "conflicting access in system; attempting to access {} mutably and immutably at the same time", 
                std::any::type_name::<T>()
            ),
            Some(Access::Write) => panic!(
                "conflicting access in system; attempting to access {} mutably twice", 
                std::any::type_name::<T>()
            ),
            None => (),
        }
    }

    unsafe fn retrieve<'r>(resources: &'r TypeMap) -> Self::Item<'r> {
        let value = resources[&TypeId::of::<T>()].get();

        // SAFETY:
        // The caller asserts that there are no conflicting accesses, and the pointer is definitely
        // valid as it was obtained directly from `UnsafeCell`. Its lifetime will be constrained
        // to the lifetime of the map it was obtained from, so it cannot dangle.
        let value = unsafe { &mut *value };

        let value = value.downcast_mut::<T>().unwrap();

        ResMut { value }
    }
}
// ANCHOR_END: ResMutSystemParam

// ANCHOR: Res
struct Res<'a, T: 'static> {
    value: &'a T,
}

impl<T: 'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}
// ANCHOR_END: Res

// ANCHOR: ResMut
struct ResMut<'a, T: 'static> {
    value: &'a mut T,
}

impl<T: 'static> Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<T: 'static> DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}
// ANCHOR_END: ResMut

struct FunctionSystem<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

// ANCHOR: System
trait System {
    fn run(&mut self, resources: &TypeMap, accesses: &mut AccessMap);
}
// ANCHOR_END: System

impl_system!();
impl_system!(T1);
impl_system!(T1, T2);
impl_system!(T1, T2, T3);
impl_system!(T1, T2, T3, T4);

trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

impl_into_system!();
impl_into_system!(T1);
impl_into_system!(T1, T2);
impl_into_system!(T1, T2, T3);
impl_into_system!(T1, T2, T3, T4);

type StoredSystem = Box<dyn System>;

// ANCHOR: Scheduler
#[derive(Default)]
struct Scheduler {
    systems: Vec<StoredSystem>,
    resources: TypeMap,
    accesses: AccessMap,
}
// ANCHOR_END: Scheduler

// ANCHOR: SchedulerImpl
impl Scheduler {
    pub fn run(&mut self) {
        for system in self.systems.iter_mut() {
            system.run(&self.resources, &mut self.accesses);
        }
        self.accesses.clear();
    }

    pub fn add_system<I, S: System + 'static>(&mut self, system: impl IntoSystem<I, System = S>) {
        self.systems.push(Box::new(system.into_system()));
    }

    pub fn add_resource<R: 'static>(&mut self, res: R) {
        let value = UnsafeCell::new(Box::new(res));

        self.resources.insert(TypeId::of::<R>(), value);
    }
}
// ANCHOR_END: SchedulerImpl
// ANCHOR_END: All
