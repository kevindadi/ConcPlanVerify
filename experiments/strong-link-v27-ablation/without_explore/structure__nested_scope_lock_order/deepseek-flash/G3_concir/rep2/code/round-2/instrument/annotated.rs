mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

#[allow(non_upper_case_globals)]
static a: Mutex<()> = Mutex::new_named("a_mutex0", ());

#[allow(non_upper_case_globals)]
static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

fn x1() {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2() {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer() {
    let h_x1 = cir_trace::spawn("x1#462", || {
        x1();
    });
    let h_x2 = cir_trace::spawn("x2#518", || {
        x2();
    });
    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn main() { cir_trace::init();
    let h_outer = cir_trace::spawn("outer#644", || {
        outer();
    });
    h_outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
