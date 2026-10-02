mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
    let result = 1;

    drop(guard_b);
    drop(guard_a);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
    let result = 1;

    drop(guard_b);
    drop(guard_a);
    result
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#587", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#625", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#707", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#814", move || t2(a2, b2));

    let result1 = h1.join().unwrap();
    let result2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
 cir_trace::finish();}
