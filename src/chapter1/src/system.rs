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
#impl<F: FnMut()> System for FunctionSystem<(), F> {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#        (self.f)()
#    }
#}
#
#impl<F: FnMut(T1), T1: 'static> System for FunctionSystem<(T1,), F> {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#        let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
#
#        (self.f)(_0)
#    }
#}
#
#impl<F: FnMut(T1, T2), T1: 'static, T2: 'static> System for FunctionSystem<(T1, T2), F> {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#        let _0 = *resources.remove(&TypeId::of::<T1>()).unwrap().downcast::<T1>().unwrap();
#        let _1 = *resources.remove(&TypeId::of::<T2>()).unwrap().downcast::<T2>().unwrap();
#
#        (self.f)(_0, _1)
#    }
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
// ANCHOR_END: All