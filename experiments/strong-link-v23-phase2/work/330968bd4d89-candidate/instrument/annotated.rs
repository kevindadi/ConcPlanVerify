mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn worker(
    name: &'static str,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
) {
    // Both workers acquire the locks in the same order (a, then b),
    // which makes deadlock impossible (R5). Mutex::lock() blocks until
    // the lock is free, satisfying R4.
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work: both locks are held at the same time here (R3).
    println!("{}: holding a and b", name);

    // Guards are dropped here, releasing both locks before finishing (R7).
}

fn main() { cir_trace::init();
    // Shared resources: a and b (R2: shared by both workers).
    let a = Arc::new(Mutex::new_named("a_mutex0#686", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#724", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("worker#806", move || worker("t1", a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("worker#923", move || worker("t2", a2, b2));

    // Main thread starts both workers and waits for both to finish (R6).
    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
