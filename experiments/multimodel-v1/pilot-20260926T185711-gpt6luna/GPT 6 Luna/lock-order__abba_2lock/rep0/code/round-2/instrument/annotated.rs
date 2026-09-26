mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

static a: Mutex<()> = Mutex::new_named("a_mutex0", ());
static b: Mutex<()> = Mutex::new_named("b_mutex0", ());

fn t1() -> i32 {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn t2() -> i32 {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn main() { cir_trace::init();
    let handle_t1 = cir_trace::spawn("handle_t1", t1);
    let handle_t2 = cir_trace::spawn("handle_t2", t2);

    let result_t1 = handle_t1.join().unwrap();
    let result_t2 = handle_t2.join().unwrap();

    println!("DONE t1={} t2={}", result_t1, result_t2);
 cir_trace::finish();}
