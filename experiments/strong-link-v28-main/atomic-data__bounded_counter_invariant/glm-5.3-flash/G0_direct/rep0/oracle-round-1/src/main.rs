mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// R2/R3/R4: each worker (w1, w2) locks the mutex m before reading or
// writing the shared counter c, and adds exactly one while holding it.
fn worker(m: &Arc<Mutex<i32>>) {
    // Acquire the mutual-exclusion lock m.
    let mut c = m.lock().unwrap();

    // R5: the counter must stay within its declared range 0..=2.
    assert!(*c >= 0, "counter below declared range");
    assert!(*c <= 1, "counter above declared range before increment");

    // R3: add exactly one to the shared counter while holding the lock.
    *c += 1;

    assert!(*c <= 2, "counter above declared range after increment");

    // Lock is released when `c` (the guard) goes out of scope.
}

fn main() { cir_trace::init();
    // R1: the supervising task (main) sets up the shared state and
    // launches the two worker threads, then waits for both to finish.

    // R2: a single shared integer counter, declared range 0..=2,
    // starting at zero. The mutex m guards the variable c.
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#1042", 0));

    // Launch worker w1.
    let m_for_w1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("worker#1125", move || worker(&m_for_w1));

    // Launch worker w2.
    let m_for_w2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("worker#1241", move || worker(&m_for_w2));

    // R1/R6: wait for both workers; every schedule terminates because
    // each worker takes the lock at most once and never blocks forever.
    w1.join().expect("worker w1 panicked");
    w2.join().expect("worker w2 panicked");

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
 cir_trace::finish();}
