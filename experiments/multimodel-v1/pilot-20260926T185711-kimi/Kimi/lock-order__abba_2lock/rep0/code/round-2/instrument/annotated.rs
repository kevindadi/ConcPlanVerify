mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<(i32, i32)>>, b: Arc<Mutex<()>>) {
    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.0 = 1;
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: Arc<Mutex<(i32, i32)>>, b: Arc<Mutex<()>>) {
    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.1 = 1;
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", (0i32, 0i32)));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    let guard = a.lock().unwrap();
    println!("DONE t1={} t2={}", guard.0, guard.1);
 cir_trace::finish();}
