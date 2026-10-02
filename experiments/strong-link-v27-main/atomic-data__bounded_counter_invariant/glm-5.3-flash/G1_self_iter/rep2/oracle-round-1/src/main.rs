mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resources (exact names from the spec):
//   m -- the mutual-exclusion lock guarding the counter
//   c -- the shared integer counter, declared range 0..=2, starts at 0
//
// Invariant (R5): c is only ever read or written while m is held, and each
// worker adds exactly 1, so c can only take the values 0, 1, 2.

// Worker role w1 (R3, R4): increments c exactly once, under the lock.
fn w1(m: &Arc<Mutex<u8>>) {
    // Lock is acquired here; every read and write of c below happens
    // while this worker holds m, so w1 and w2 can never update c
    // at the same time (R4).
    let mut c = m.lock().expect("lock m poisoned");
    *c += 1; // single read-modify-write of c while holding m
    // Lock is released here when the guard is dropped.
}

// Worker role w2 (R3, R4): identical to w1.
fn w2(m: &Arc<Mutex<u8>>) {
    let mut c = m.lock().expect("lock m poisoned");
    *c += 1;
}

// Supervising task (R1): launches w1 and w2, waits for both to finish.
fn supervisor() {
    // c starts at zero, within its declared range 0..=2 (R2).
    let m: Arc<Mutex<u8>> = Arc::new(Mutex::new_named("res_mutex0#1146", 0));

    let m_for_w1 = Arc::clone(&m);
    let m_for_w2 = Arc::clone(&m);

    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || w1(&m_for_w1))
        .expect("failed to spawn w1");

    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || w2(&m_for_w2))
        .expect("failed to spawn w2");

    // Wait for both workers (R1, R6). Neither worker blocks on anything
    // except m, and each holds m only for a finite, non-blocking critical
    // section, so every interleaving terminates.
    h1.join().expect("worker w1 panicked");
    h2.join().expect("worker w2 panicked");

    // done: set to 1 once both workers have finished (R7).
    let done = 1;
    println!("DONE done={}", done);
}

fn main() { cir_trace::init();
    // The program starts the supervising task (R1) and waits for it,
    // so the process exits only after the line has been printed (R7).
    let sup = thread::Builder::new()
        .name("supervisor".to_string())
        .spawn(supervisor)
        .expect("failed to spawn supervisor");

    sup.join().expect("supervisor panicked");
 cir_trace::finish();}
