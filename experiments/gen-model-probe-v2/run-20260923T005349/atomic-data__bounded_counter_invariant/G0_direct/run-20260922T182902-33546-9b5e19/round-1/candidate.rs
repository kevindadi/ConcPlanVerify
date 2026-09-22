use std::sync::{Arc, Mutex};
use std::thread;

// R2/R4: `c` is the shared integer counter (declared range 0..=2, starts at 0).
// It is stored inside the mutex `m`, so `c` can only be read or written while
// the mutual-exclusion lock `m` is held.

// Worker role w1 (R3): adds exactly one to `c` while holding `m`.
fn w1(m: Arc<Mutex<i64>>) {
    let mut c = m.lock().unwrap(); // acquire m; guard is the only access path to c
    *c += 1; // read+write of c performed under the lock
} // lock released here

// Worker role w2 (R3): adds exactly one to `c` while holding `m`.
fn w2(m: Arc<Mutex<i64>>) {
    let mut c = m.lock().unwrap(); // acquire m; the two workers can never update concurrently
    *c += 1;
} // lock released here

fn main() {
    // R1: supervising task. Create shared resources m (lock) and c (counter = 0).
    let m = Arc::new(Mutex::new(0i64)); // c starts at 0, within declared range 0..=2

    // Launch the two worker threads.
    let m_for_w1 = Arc::clone(&m);
    let h_w1 = thread::spawn(move || w1(m_for_w1));

    let m_for_w2 = Arc::clone(&m);
    let h_w2 = thread::spawn(move || w2(m_for_w2));

    // Wait for both workers to finish (R1). Joining cannot deadlock: each
    // worker only takes the lock once, performs one increment, and releases
    // it, so every interleaving terminates (R6).
    h_w1.join().unwrap();
    h_w2.join().unwrap();

    // R5: with exactly two locked increments from 0, c == 2, still within 0..=2.
    {
        let c = m.lock().unwrap();
        debug_assert!((0..=2).contains(&*c));
    }

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
}
