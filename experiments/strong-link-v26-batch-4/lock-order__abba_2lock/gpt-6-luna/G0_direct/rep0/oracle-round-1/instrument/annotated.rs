mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) -> usize {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();

    let value = 1;

    drop(_b);
    drop(_a);
    value
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) -> usize {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();

    let value = 1;

    drop(_b);
    drop(_a);
    value
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#433", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#471", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("t1#560", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("t2#676", move || t2(&a2, &b2));

    let result_t1 = handle_t1.join().unwrap();
    let result_t2 = handle_t2.join().unwrap();

    println!("DONE t1={} t2={}", result_t1, result_t2);
 cir_trace::finish();}
