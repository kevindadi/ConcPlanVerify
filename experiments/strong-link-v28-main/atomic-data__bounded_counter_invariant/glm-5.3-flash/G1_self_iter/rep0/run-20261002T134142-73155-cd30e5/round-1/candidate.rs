use std::sync::{Arc, Mutex};
use std::thread;

// Shared resource: c — declared range 0..=2, starts at 0.
// Guarded by m, the mutual-exclusion lock.

fn worker(name: &'static str, m: Arc<Mutex<u8>>) {
    // R3/R4: add exactly one to c while holding the lock m.
    // The lock guard is held across the entire read-modify-write,
    // so w1 and w2 can never update c at the same time.
    let mut c = m.lock().expect("lock poisoned");
    *c += 1;
    // Guard drops here, releasing m.
    let _ = name; // w1 / w2 role label
}

fn main() {
    // R1: supervising task (main) launches w1 and w2 and waits for both.
    let m = Arc::new(Mutex::new(0u8)); // c starts at 0, range 0..=2

    let m_w1 = Arc::clone(&m);
    let w1 = thread::spawn(move || worker("w1", m_w1));

    let m_w2 = Arc::clone(&m);
    let w2 = thread::spawn(move || worker("w2", m_w2));

    // R6: joining both workers terminates under every schedule.
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // R4: final read of c also happens under the lock.
    let c = *m.lock().expect("lock poisoned");
    debug_assert!((0..=2).contains(&c)); // R5: value stays in declared range

    // R7 [U]: as specified this would print "DONE done=1", but R2+R3
    // force the final value to be 2. Print the true final value.
    println!("DONE done={}", c);
}
