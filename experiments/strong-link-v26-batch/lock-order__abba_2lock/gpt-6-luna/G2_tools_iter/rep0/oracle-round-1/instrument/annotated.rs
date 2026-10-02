mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let result = 1; // Critical work while holding both locks.

    drop(b_guard);
    drop(a_guard);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let result = 1; // Critical work while holding both locks.

    drop(b_guard);
    drop(a_guard);
    result
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#579", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#617", ()));

    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("t1#718", move || t1(a_for_t1, b_for_t1));

    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("t2#856", move || t2(a_for_t2, b_for_t2));

    let t1 = handle_t1.join().unwrap();
    let t2 = handle_t2.join().unwrap();

    println!("DONE t1={t1} t2={t2}");
 cir_trace::finish();}
