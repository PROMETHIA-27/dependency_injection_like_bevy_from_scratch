// ANCHOR: All
#use std::collections::HashMap;
#use std::marker::PhantomData;
#use std::any::{Any, TypeId};
#struct FunctionSystem<Input, F> {
#    f: F,
#    marker: PhantomData<fn() -> Input>,
#}
#
#trait System {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
#}
#
#trait IntoSystem<Input> {
#    type System: System;
#
#    fn into_system(self) -> Self::System;
#}
#
#impl<F: FnMut()> IntoSystem<()> for F {
#    type System = FunctionSystem<(), Self>;
#
#    fn into_system(self) -> Self::System {
#        FunctionSystem {
#            f: self,
#            marker: Default::default(),
#        }
#    }
#}
#
#impl<F: FnMut(T1,), T1: 'static> IntoSystem<(T1,)> for F {
#    type System = FunctionSystem<(T1,), Self>;
#
#    fn into_system(self) -> Self::System {
#        FunctionSystem {
#            f: self,
#            marker: Default::default(),
#        }
#    }
#}
#
#impl<F: FnMut(T1, T2), T1: 'static, T2: 'static> IntoSystem<(T1, T2)> for F {
#    type System = FunctionSystem<(T1, T2), Self>;
#
#    fn into_system(self) -> Self::System {
#        FunctionSystem {
#            f: self,
#            marker: Default::default(),
#        }
#    }
#}
#
#type StoredSystem = Box<dyn System>;
#
#struct Scheduler {
#    systems: Vec<StoredSystem>,
#    resources: HashMap<TypeId, Box<dyn Any>>,
#}
#
#impl Scheduler {
#    pub fn run(&mut self) {
#        for system in self.systems.iter_mut() {
#            system.run(&mut self.resources);
#        }
#    }
#
#    pub fn add_system<I, S: System + 'static>(&mut self, system: impl IntoSystem<I, System = S>) {
#        self.systems.push(Box::new(system.into_system()));
#    }
#
#    pub fn add_resource<R: 'static>(&mut self, res: R) {
#        self.resources.insert(TypeId::of::<R>(), Box::new(res));
#    }
#}
#macro_rules! impl_system {
#    ($($params:ident),*) => {
#        #[allow(unused_variables)]
#        #[allow(non_snake_case)]
#        impl<F: FnMut($($params),*), $($params : 'static),*> System for FunctionSystem<($($params ,)*), F> {
#            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#                $(
#                    let $params = *resources.remove(&TypeId::of::<$params>()).unwrap().downcast::<$params>().unwrap();
#                )*
#
#                (self.f)($($params),*)
#            }
#        }
#    };
#}
#macro_rules! call_n_times {
#    ($target:ident, 1) => {
#        $target!();
#    };
#
#    ($target:ident, 2) => {
#        $target!(T1);
#        call_n_times!($target, 1);
#    };
#    
#    ($target:ident, 3) => {
#        $target!(T1, T2);
#        call_n_times!($target, 2);
#    };
#
#    // etc.
#}
// ANCHOR_END: All