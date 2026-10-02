mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();

    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();

    1
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#341", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#379", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = cir_trace::spawn("t1#466", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = cir_trace::spawn("t2#578", move || t2(a2, b2));

    let result1 = handle1.join().unwrap();
    let result2 = handle2.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
 cir_trace::finish();}
