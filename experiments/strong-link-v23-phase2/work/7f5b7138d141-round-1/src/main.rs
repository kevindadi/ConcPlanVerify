mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    drop(guard_b);
    drop(guard_a);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#395", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#433", ()));

    let a_t1 = Arc::clone(&a);
    let b_t1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("t1#526", move || t1(&a_t1, &b_t1));

    let a_t2 = Arc::clone(&a);
    let b_t2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("t2#650", move || t2(&a_t2, &b_t2));

    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
