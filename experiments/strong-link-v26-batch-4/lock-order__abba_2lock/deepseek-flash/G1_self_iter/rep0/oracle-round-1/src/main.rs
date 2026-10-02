mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    // Consistent lock ordering: always acquire a, then b.
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    // Critical work while holding both locks.
    let _ = 1 + 1;

    drop(gb);
    drop(ga);
    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    // Same lock ordering as t1 prevents circular wait/deadlock.
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    // Critical work while holding both locks.
    let _ = 1 + 1;

    drop(gb);
    drop(ga);
    1
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#655", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#693", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::Builder::new()
        .name("t1".to_string())
        .spawn(move || t1(a1, b1))
        .unwrap();

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::Builder::new()
        .name("t2".to_string())
        .spawn(move || t2(a2, b2))
        .unwrap();

    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}
