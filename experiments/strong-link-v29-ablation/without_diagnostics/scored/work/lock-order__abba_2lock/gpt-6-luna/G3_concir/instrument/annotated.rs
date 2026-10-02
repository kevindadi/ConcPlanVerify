mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let result = 1;
    drop(b_guard);
    drop(a_guard);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let result = 1;
    drop(b_guard);
    drop(a_guard);
    result
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#485", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#523", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = crate::cir_trace::spawn("t1#612", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = crate::cir_trace::spawn("t2#726", move || t2(a2, b2));

    let result1 = t1_handle.join().unwrap();
    let result2 = t2_handle.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
 crate::cir_trace::finish();}
