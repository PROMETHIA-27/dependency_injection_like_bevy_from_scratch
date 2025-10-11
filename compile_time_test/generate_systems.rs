#![feature(random)]

use std::random::*;

const COUNT: usize = 5000;

fn main() {
    let args = std::env::args();
    let call = matches!(args.skip(1).next().as_ref().map(String::as_str), Some("call"));

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

const TYPES: &[&str] = &[
    "i32",
    "usize",
    "&'static str",
];

fn generate_header(out: &mut String) {
    out.push_str("#![allow(unused)]\n");
    out.push_str("use crate::{Res, Scheduler, SystemParam};\n\n");
}

fn generate_system_adds(out: &mut String) {
    out.push_str("pub fn add_systems(sched: &mut Scheduler) {\n");

    for i in 0..COUNT {
        out.push_str(&format!("\tsched.add_system(system{i});\n"));
    }

    out.push_str("}\n\n");
}

fn generate_system_calls(out: &mut String) {
    out.push_str("pub fn call_systems(sched: &mut Scheduler) {\n");

    out.push_str("\tlet arg0: Res<i32> = Res::retrieve(&sched.resources);\n");
    out.push_str("\tlet arg1: Res<usize> = Res::retrieve(&sched.resources);\n");
    out.push_str("\tlet arg2: Res<&'static str> = Res::retrieve(&sched.resources);\n");

    out.push_str("\tlet arg0: &i32 = &arg0;\n");
    out.push_str("\tlet arg1: &usize = &arg1;\n");
    out.push_str("\tlet arg2: &str = arg2.clone();\n");

    for i in 0..COUNT {
        out.push_str(&format!("\tsystem{i}(arg0, arg1, arg2);\n"));
    }

    out.push_str("}\n\n");
}

fn generate_system(out: &mut String, index: usize) {
    out.push_str(&format!("pub fn system{index}("));

    let arg_count = random::<u8>(..) % 8;

    for i in 0..arg_count {
        let ty_index = random::<u8>(..) % TYPES.len() as u8;
        let ty = TYPES[ty_index as usize];

        out.push_str(&format!("_arg{i}: Res<{ty}>, "));
    }    

    out.push_str(") { println!(\"Hello, world!\"); }\n\n");
}

fn generate_call_system(out: &mut String, index: usize) {
    out.push_str(&format!("pub fn system{index}("));
    
    out.push_str("_arg0: &i32, ");
    out.push_str("_arg1: &usize, ");
    out.push_str("_arg2: &str, ");

    out.push_str(") { println!(\"Hello, world!\"); }\n\n");
}