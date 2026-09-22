mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Shared state protected by the lock `m`.
struct Shared {
    /// Shared counter `c`: declared range 0..=2, starts at 0 (R2).
    /// The program logic maintains the stronger invariant c <= 1 (R5).
    c: i32,
}

/// Worker body used by both w1 and w2.
fn worker(m: &Mutex<Shared>) {
    // R4: hold the mutual-exclusion lock `m` for every read/write of `c`,
    // so the two workers never update it at the same time.
    let mut guard = m.lock().unwrap();

    // R3 + R5: add one only when the result stays within the required
    // upper limit, so `c` never exceeds one in any reachable state,
    // even though its declared range allows up to two.
    if guard.c + 1 <= 1 {
        guard.c += 1;
    }
}

fn main() { cir_trace::init();
    // R2: `m` is the shared lock; `c` is the shared counter, starting at 0.
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { c: 0 }));

    // R1: the supervising task launches worker threads w1 and w2.
    let m_w1 = Arc::clone(&m);
    let w1 = cir_trace::spawn("w1", move || worker(&m_w1));

    let m_w2 = Arc::clone(&m);
    let w2 = cir_trace::spawn("w2", move || worker(&m_w2));

    // R1 + R6: wait for both workers. Every schedule terminates because
    // each worker performs exactly one lock, one bounded check, and one
    // unlock -- no loops, no waiting, no nested locking.
    w1.join().unwrap();
    w2.join().unwrap();

    // R7: exactly one worker increments `c` (whichever acquires `m` first),
    // so the final value is deterministically 1.
    let guard = m.lock().unwrap();
    println!("DONE done={}", guard.c);
 cir_trace::finish();}
