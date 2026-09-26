mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn t1(a: &Mutex<(i32, i32)>, b: &Mutex<()>) {
    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.0 = 1;
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: &Mutex<(i32, i32)>, b: &Mutex<()>) {
    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.1 = 1;
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", (0i32, 0i32));
    let b = Mutex::new_named("b_mutex0", ());

    thread::scope(|s| {
        s.spawn(|| t1(&a, &b));
        s.spawn(|| t2(&a, &b));
    });

    let guard = a.lock().unwrap();
    println!("DONE t1={} t2={}", guard.0, guard.1);
 cir_trace::finish();}
