use std::sync::{Arc, Mutex};
use std::thread;

// Shared resources:
//   m — the mutual-exclusion lock (Mutex) guarding c.
//   c — the shared integer counter, declared range 0..=2, starting at 0.
// Every read or write of c happens while the worker holds m (R4).
type Counter = Arc<Mutex<u8>>;

// Worker role w1: adds exactly one to c while holding the lock m (R3).
fn w1(c: Counter) {
    let mut guard = c.lock().unwrap(); // acquire m
    *guard += 1;                       // read-modify-write of c under m
    // lock m released here
}

// Worker role w2: identical to w1.
fn w2(c: Counter) {
    let mut guard = c.lock().unwrap(); // acquire m
    *guard += 1;                       // read-modify-write of c under m
    // lock m released here
}

fn main() {
    // Supervising task (R1): launches w1 and w2, then waits for both to finish.
    // c starts at 0; its declared range is 0..=2 (R2).
    let c: Counter = Arc::new(Mutex::new(0));

    let h1 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w1(c))
    };
    let h2 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w2(c))
    };

    // Wait for both workers (R1). Each worker does a bounded, non-retrying
    // critical section, so every schedule/interleaving terminates (R6).
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Invariant check (R5): after both workers, c == 2, which is within 0..=2.
    // At every intermediate state c is 0, 1, or 2 — always in range, because
    // each increment happens atomically under m and there are exactly two.
    let final_value = *c.lock().unwrap();
    debug_assert!((0..=2).contains(&final_value));

    // R7: print exactly this line, then exit.
    println!("DONE done=1");
}
