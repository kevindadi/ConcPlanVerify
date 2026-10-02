mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn critical_work(a: &Mutex<()>, b: &Mutex<()>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    // Both locks are held while the critical work is performed.
    let result = 1;

    drop(b_guard);
    drop(a_guard);
    result
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#526", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#564", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let worker1 = cir_trace::spawn("t1#651", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let worker2 = cir_trace::spawn("t2#763", move || t2(a2, b2));

    let result1 = worker1.join().unwrap();
    let result2 = worker2.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
 cir_trace::finish();}
