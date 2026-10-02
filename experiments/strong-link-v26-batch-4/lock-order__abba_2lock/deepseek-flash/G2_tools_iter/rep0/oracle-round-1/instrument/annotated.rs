mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    // Critical work while holding both locks.

    drop(guard_b);
    drop(guard_a);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    // Critical work while holding both locks.

    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#491", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#529", ()));

    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let t1_handle = cir_trace::spawn("t1#630", move || t1(&a_for_t1, &b_for_t1));

    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);
    let t2_handle = cir_trace::spawn("t2#770", move || t2(&a_for_t2, &b_for_t2));

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
