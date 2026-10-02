mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> u8 {
    // Acquire shared locks in the same order in both workers.
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work requiring both a and b.
    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> u8 {
    // Acquire shared locks in the same order in both workers.
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work requiring both a and b.
    1
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#579", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#617", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle_t1 = cir_trace::spawn("t1#706", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle_t2 = cir_trace::spawn("t2#820", move || t2(a2, b2));

    let result_t1 = handle_t1.join().unwrap();
    let result_t2 = handle_t2.join().unwrap();

    println!("DONE t1={result_t1} t2={result_t2}");
 cir_trace::finish();}
