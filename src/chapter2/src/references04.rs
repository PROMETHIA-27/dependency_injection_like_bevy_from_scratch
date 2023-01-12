#use std::any::{Any, TypeId};
#use std::collections::HashMap;
#use std::marker::PhantomData;
#
#struct Res<'a, T: 'static> {
#    value: &'a T,
#}
#
#macro_rules! impl_system {
#    (
#        $(
#            $($params:ident),+
#        )?
#    ) => {
#        #[allow(non_snake_case)]
#        #[allow(unused)]
#        impl<
#            'a,
#            F: FnMut(
#                $( $($params),+ )?
#            )
#            $(, $($params: SystemParam<'a>),+ )?
#        > System<'a> for FunctionSystem<($( $($params,)+ )?), F> {
#            fn run(&mut self, resources: &'a mut HashMap<TypeId, Box<dyn Any>>) {
#                $($(
#                    let $params = $params::retrieve(resources);
#                )+)?
#
#                (self.f)(
#                    $($($params),+)?
#                );
#            }
#        }
#    }
#}
#
#macro_rules! impl_into_system {
#    (
#        $($(
#                $params:ident
#        ),+)?
#    ) => {
#        impl<'a, F: FnMut($($($params),+)?) $(, $($params: SystemParam<'a>),+ )?> IntoSystem<'a, ( $($($params,)+)? )> for F {
#            type System = FunctionSystem<( $($($params,)+)? ), Self>;
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
#struct FunctionSystem<Input, F> {
#    f: F,
#    marker: PhantomData<fn() -> Input>,
#}
#
#trait System<'a> {
#    fn run(&mut self, resources: &'a mut HashMap<TypeId, Box<dyn Any>>);
#}
#
#impl_system!();
#impl_system!(T1);
#impl_system!(T1, T2);
#impl_system!(T1, T2, T3);
#impl_system!(T1, T2, T3, T4);
#
#trait IntoSystem<'a, Input> {
#    type System: System<'a>;
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
#type StoredSystem = Box<dyn for<'a> System<'a>>;
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
#    pub fn add_system<I, S: for<'a> System<'a> + 'static>(&mut self, system: impl for<'a> IntoSystem<'a, I, System = S>) {
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
#
#fn foo(int: Res<i32>) {
#    println!("int! {}", int.value);    
#}