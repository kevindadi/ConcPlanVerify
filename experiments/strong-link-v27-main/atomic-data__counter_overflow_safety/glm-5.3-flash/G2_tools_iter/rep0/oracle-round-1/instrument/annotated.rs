mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared counter `c` is a u8 whose declared range is 0..=2 and which starts at 0 (R2).
// It is protected by the mutex `m` (R4).

// Worker role: w1 and w2 both run this function.
fn worker(id: &'static str, m: Arc<Mutex<u8>>) {
    // Hold the mutual-exclusion lock for the whole read-modify-write (R4).
    let mut c = m.lock().unwrap();

    // Read the counter while holding the lock.
    // Increment only if the result stays within the required upper limit (R3).
    // The required upper limit is 1, so in every reachable state c never
    // exceeds 1, even though its declared range allows up to 2 (R5).
    if *c < 1 {
        *c += 1;
    }
    debug_assert!(*c <= 2, "counter out of declared range 0..=2");

    // Lock is released here when the guard `c` is dropped.
    let _ = id; // w1 / w2 label, kept for traceability
}

fn main() { cir_trace::init();
    // Supervising task: launches the two workers and waits for both (R1).
    let m = Arc::new(Mutex::new_named("m_mutex0#1002", 0u8)); // c starts at 0 (R2)

    let m_w1 = Arc::clone(&m);
    let m_w2 = Arc::clone(&m);

    // Worker w1
    let h1 = cir_trace::spawn("worker#1129", move || worker("w1", m_w1));
    // Worker w2
    let h2 = cir_trace::spawn("worker#1202", move || worker("w2", m_w2));

    // Wait for both workers to finish. Each worker attempts its increment
    // exactly once and never loops or blocks indefinitely, so every
    // schedule and interleaving terminates (R6).
    h1.join().unwrap();
    h2.join().unwrap();

    // Exactly one of the two workers performed the increment, so the final
    // value is 1 (R5, R7).
    let c = m.lock().unwrap();
    println!("DONE done={}", *c);
 cir_trace::finish();}
