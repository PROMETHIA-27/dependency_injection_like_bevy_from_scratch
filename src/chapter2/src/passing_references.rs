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
#
#trait SystemParam {
#    type Item<'new>;
#
#    fn retrieve<'r>(resources: &'r HashMap<TypeId, Box<dyn Any>>) -> Self::Item<'r>;
#}
#
#struct Res<'a, T: 'static> {
#    value: &'a T,
#}
#
#impl<'res, T: 'static> SystemParam for Res<'res, T> {
#    type Item<'new> = Res<'new, T>;
#
#    fn retrieve<'r>(resources: &'r HashMap<TypeId, Box<dyn Any>>) -> Self::Item<'r> {
#        Res { value: resources.get(&TypeId::of::<T>()).unwrap().downcast_ref().unwrap() }
#    }
#}
#
#impl<F, T1: SystemParam, T2: SystemParam> System for FunctionSystem<(T1, T2), F> 
#where
#    // for any two arbitrary lifetimes, a mutable reference to F with lifetime 'a
#    // implements FnMut taking parameters of lifetime 'b
#    for<'a, 'b> &'a mut F:
#        FnMut(T1, T2) +
#        FnMut(<T1 as SystemParam>::Item<'b>, <T2 as SystemParam>::Item<'b>)
#{
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#        // necessary to tell rust exactly which impl to call; it gets a bit confused otherwise
#        fn call_inner<T1, T2>(
#            mut f: impl FnMut(T1, T2),
#            _0: T1,
#            _1: T2
#        ) {
#            f(_0, _1)
#        }
#
#        let _0 = T1::retrieve(resources);
#        let _1 = T2::retrieve(resources);
#
#        call_inner(&mut self.f, _0, _1)
#    }
#}
#
#impl<F: FnMut(T1, T2), T1: SystemParam, T2: SystemParam> IntoSystem<(T1, T2)> for F 
#where
#    for<'a, 'b> &'a mut F: 
#        FnMut(T1, T2) + 
#        FnMut(<T1 as SystemParam>::Item<'b>, <T2 as SystemParam>::Item<'b>)
#{
#    type System = FunctionSystem<(T1, T2), Self>;
#
#    fn into_system(self) -> Self::System {
#        FunctionSystem {
#            f: self,
#            marker: Default::default(),
#        }
#    }
#}
// ANCHOR_END: All