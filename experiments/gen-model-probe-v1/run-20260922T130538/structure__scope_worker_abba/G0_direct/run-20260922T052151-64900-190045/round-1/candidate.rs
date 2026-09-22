use std::sync::{Arc, Mutex};
use std::thread;

// Each worker contends for the same two mutexes A and B.
// Both workers acquire them in the same order (A first, then B),
// so no wait cycle can form and every interleaving terminates.
// A worker that cannot take a held mutex blocks in `lock()` until
// it becomes free. Each mutex is held by at most one worker at a
// time, and each guard is released when the worker's work finishes.
fn worker(a: &Mutex<u64>, b: &Mutex<u64>) {
    let mut guard_a = a.lock().unwrap(); // acquire A (blocks if held)
    let mut guard_b = b.lock().unwrap(); // acquire B (blocks if held)

    // Critical section: both A and B are held simultaneously.
    *guard_a += 1;
    *guard_b += 1;

    // `guard_b` and `guard_a` are dropped here, releasing each
    // mutex exactly once now that the work is finished.
}

fn main() {
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));

    // Start a group of two workers sharing both mutexes.
    let mut workers = Vec::new();
    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        workers.push(thread::spawn(move || worker(&a, &b)));
    }

    // The group finishes only after both workers have completed.
    for w in workers {
        w.join().unwrap();
    }

    println!("DONE done=1");
}
