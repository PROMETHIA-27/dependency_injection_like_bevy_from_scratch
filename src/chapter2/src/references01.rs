#use std::any::{Any, TypeId};
#use std::collections::HashMap;
#use std::marker::PhantomData;
#
#struct FunctionSystem<Input, F> {
#    f: F,
#    marker: PhantomData<fn() -> Input>,
#}
#
#trait System {
#    fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>);
#}
#
#impl_system!();
#impl_system!(T1);
#impl_system!(T1, T2);
#impl_system!(T1, T2, T3);
#impl_system!(T1, T2, T3, T4);
#
#trait IntoSystem<Input> {
#    type System: System;
#
#    fn into_system(self) -> Self::System;
#}
#
#impl_into_system!();
#impl_into_system!(T1);
#impl_into_system!(T1, T2);
#impl_into_system!(T1, T2, T3);
#impl_into_system!(T1, T2, T3, T4);
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
#fn main() {
#    let mut scheduler = Scheduler {
#        systems: vec![],
#        resources: HashMap::default(),
#    };
#
#    scheduler.add_system(foo);
#    scheduler.add_resource(12i32);
#
#    scheduler.run();
#}