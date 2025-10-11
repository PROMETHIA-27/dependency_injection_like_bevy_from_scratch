#![feature(random)]

use std::random::*;

const COUNT: usize = 5000;

fn main() {
    let args = std::env::args();
    let call = matches!(
        args.skip(1).next().as_ref().map(String::as_str),
        Some("call")
    );

    let mut out = String::new();

    generate_header(&mut out);

    if call {
        generate_system_calls(&mut out);
    } else {
        generate_system_adds(&mut out);
    }

    for i in 0..COUNT {
        if call {
            generate_call_system(&mut out, i);
        } else {
            generate_system(&mut out, i);
        }
    }

    println!("{out}");
}

const TYPES: &[&str] = &["i32", "usize", "&'static str"];

fn generate_header(out: &mut String) {
    out.push_str("#![allow(unused, non_camel_case_types)]\n");
    out.push_str("use crate::{Res, Scheduler, FunkySystem, StoredSystem};\n");
    out.push_str("use std::{any::{Any, TypeId}, cell::RefCell, collections::HashMap, marker::PhantomData};\n\n");
}

fn generate_system_adds(out: &mut String) {
    out.push_str("pub fn add_systems(sched: &mut Scheduler) {\n");

    // for i in 0..COUNT {
    //     out.push_str(&format!("\tsched.add_system(system{i});\n"));
    // }

    out.push_str("}\n\n");
}

fn generate_system_calls(out: &mut String) {
    out.push_str("pub fn call_systems(sched: &mut Scheduler) {\n");

    for i in 0..COUNT {
        out.push_str(&format!("\tcall_system{i}(sched);\n"));
    }

    out.push_str("}\n\n");
}

fn generate_system(out: &mut String, index: usize) {
    let arg_count = random::<u8>(..) % 8;
    // let arg_count = 0;

    let mut sys_args = String::new();
    let mut arg_declares = String::new();
    let mut arg_passes = String::new();

    for i in 0..arg_count {
        let ty_index = random::<u8>(..) % TYPES.len() as u8;
        let ty = TYPES[ty_index as usize];

        sys_args.push_str(&format!("_arg{i}: Res<{ty}>, "));
        arg_declares.push_str(&format!("\t\tlet arg{i} = Res::<{ty}>::get(resources);\n"));
        arg_passes.push_str(&format!("arg{i}, "));
    }

    out.push_str(&format!(
        "#[allow(non_upper_case_globals)]
pub static system{index}: StoredSystem = __system_caller{index};
// pub static system{index}: StoredSystem = &__system_caller{index};

fn __system_caller{index}(resources: &mut HashMap<TypeId, Box<dyn Any>>) {{
{arg_declares}
    __system_internal{index}({arg_passes});
}}

pub fn __system_internal{index}({sys_args}) {{ println!(\"Hello, world!\"); }}\n\n"
    ));
}

fn generate_call_system(out: &mut String, index: usize) {
    let arg_count = random::<u8>(..) % 8;

    let mut sys_args = String::new();
    let mut arg_declares = String::new();
    let mut arg_passes = String::new();

    for i in 0..arg_count {
        let ty_index = random::<u8>(..) % TYPES.len() as u8;
        let ty = TYPES[ty_index as usize];

        sys_args.push_str(&format!("_arg{i}: Res<{ty}>, "));
        arg_declares.push_str(&format!("\tlet arg{i} = Res::<{ty}>::get(&sched.resources);\n"));
        arg_passes.push_str(&format!("arg{i}, "));
    }

    out.push_str(&format!(
"pub fn call_system{index}(sched: &mut Scheduler) {{
{arg_declares}
    system{index}({arg_passes});
}}
    
pub fn system{index}({sys_args}) {{ 
    println!(\"Hello, world!\"); 
}}\n\n"
));
}
