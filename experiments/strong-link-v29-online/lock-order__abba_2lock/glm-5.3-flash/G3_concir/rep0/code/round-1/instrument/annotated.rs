mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // Lock a, then b (same order as t2, so no deadlock is possible).
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    // Critical work while holding both locks.
    // Unlock b, then a (guards drop in reverse acquisition order).
    drop(_guard_b);
    drop(_guard_a);
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // Lock a, then b (same order as t1, so no deadlock is possible).
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    // Critical work while holding both locks.
    // Unlock b, then a (guards drop in reverse acquisition order).
    drop(_guard_b);
    drop(_guard_a);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#791", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#829", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = crate::cir_trace::spawn("t1#916", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = crate::cir_trace::spawn("t2#1030", move || t2(&a2, &b2));

    handle1.join().expect("t1 panicked");
    handle2.join().expect("t2 panicked");

    println!("DONE t1=1 t2=1");
 crate::cir_trace::finish();}
