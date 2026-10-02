mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

static a: Mutex<()> = Mutex::new_named("a_mutex0", ());
static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

fn x1() {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn x2() {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn outer() {
    thread::scope(|s| {
        s.spawn(x1);
        s.spawn(x2);
    });
}

fn main() { cir_trace::init();
    let outer_handle = cir_trace::spawn("outer_handle#492", outer);
    outer_handle.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
