mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared counter `c`: declared to range from 0 to 2 and starts at 0 (R2).
// Invariant (R5): in every reachable state, c never exceeds 1, even though
// its declared range would allow up to 2. To preserve this invariant, the
// effective upper limit for any update is 1, so a worker adds 1 only when
// the result stays at or below that limit (R3).

// w1 / w2 body: each worker performs its read-modify-write of the shared
// counter while holding the mutual-exclusion lock `m` (R4), so the two
// workers never read or update `c` at the same time.
fn worker(m: &Arc<Mutex<u8>>) {
    // Lock is acquired here and held across the read and the write (R4).
    let mut c = m.lock().unwrap();
    // R3: add 1 only when doing so keeps the value within the required
    // upper limit (1, per invariant R5).
    if *c < 1 {
        *c += 1;
    }
    // Lock is released here, when the guard is dropped.
} // w2

fn main() { cir_trace::init();
    // `m` is the mutual-exclusion lock; the data it guards is the shared
    // counter `c`, declared with range 0..=2 and initialized to 0 (R2).
    let m: Arc<Mutex<u8>> = Arc::new(Mutex::new_named("res_mutex0#1161", 0u8));

    let m_for_w1 = Arc::clone(&m);
    let m_for_w2 = Arc::clone(&m);

    // R1: the supervising task (main) launches the two worker threads
    // w1 and w2.
    let h1 = cir_trace::spawn("worker#1346", move || worker(&m_for_w1)); // w1
    let h2 = cir_trace::spawn("worker#1407", move || worker(&m_for_w2)); // w2

    // R1 (cont.) and R6: wait for both workers; every interleaving
    // terminates because neither worker loops or blocks indefinitely.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Exactly one worker finds c == 0 and increments it to 1; the other
    // finds c == 1 and leaves it unchanged, so c is always 1 here (R5, R7).
    let c = *m.lock().unwrap();
    println!("DONE done={}", c);
 cir_trace::finish();}
