#![allow(non_upper_case_globals)]

mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

static a: Mutex<()> = Mutex::new_named("a_mutex0", ());
static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

fn t1() {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
}

fn t2() {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
}

fn main() { cir_trace::init();
    let t1_handle = cir_trace::spawn("t1_handle#458", t1);
    let t2_handle = cir_trace::spawn("t2_handle#497", t2);

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
