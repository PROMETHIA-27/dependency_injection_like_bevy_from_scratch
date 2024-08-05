#use std::collections::HashMap;
#use std::any::{Any, TypeId};
#use std::marker::PhantomData;
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
#// struct ResMut<'a, T: 'static> {
#//     value: &'a mut T,
#// }
#
#// impl<'a, T: 'static> SystemParam<'a> for ResMut<'a, T> {
#//     fn retrieve(resources: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Self {
#//         let value = resources.get_mut(&TypeId::of::<T>()).unwrap().downcast_mut::<T>().unwrap();
#//         ResMut { value }
#//     }
#// }
#
#// struct ResOwned<T: 'static> {
#//     value: T
#// }
#
#// impl<'a, T: 'static> SystemParam<'a> for ResOwned<T> {
#//     fn retrieve(resources: &'a mut HashMap<TypeId, Box<dyn Any>>) -> Self {
#//         let value = *resources.remove(&TypeId::of::<T>()).unwrap().downcast::<T>().unwrap();
#//         ResOwned { value }
#//     }
#// }
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
#macro_rules! impl_system {
#    (
#        $($params:ident),*
#    ) => {
#        #[allow(non_snake_case)]
#        #[allow(unused)]
#        impl<F, $($params: SystemParam),*> System for FunctionSystem<($($params,)*), F> 
#            where
#                for<'a, 'b> &'a mut F: 
#                    FnMut( $($params),* ) + 
#                    FnMut( $(<$params as SystemParam>::Item<'b>),* )
#        {
#            fn run(&mut self, resources: &mut HashMap<TypeId, Box<dyn Any>>) {
#                fn call_inner<$($params),*>(
#                    mut f: impl FnMut($($params),*),
#                    $($params: $params),*
#                ) {
#                    f($($params),*)
#                }
#
#                $(
#                    let $params = $params::retrieve(resources);
#                )*
#
#                call_inner(&mut self.f, $($params),*)
#            }
#        }
#    }
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
#macro_rules! impl_into_system {
#    (
#        $($params:ident),*
#    ) => {
#        impl<F, $($params: SystemParam),*> IntoSystem<($($params,)*)> for F 
#            where
#                for<'a, 'b> &'a mut F: 
#                    FnMut( $($params),* ) + 
#                    FnMut( $(<$params as SystemParam>::Item<'b>),* )
#        {
#            type System = FunctionSystem<($($params,)*), Self>;
#
#            fn into_system(self) -> Self::System {
#                FunctionSystem {
#                    f: self,
#                    marker: Default::default(),
#                }
#            }
#        }
#    }
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
#    scheduler.add_resource(7u32);
#
#    scheduler.run();
#}