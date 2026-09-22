use std::sync::{Arc, Mutex};
use std::thread;

// Distinct payload types so the two mutexes are genuinely different lock
// types. This keeps the lock-ordering discipline explicit and avoids any
// aliasing ambiguity between the two locks.
struct Counter1(i32);
struct Counter2(i32);

fn main() {
    let t1 = Arc::new(Mutex::new(Counter1(0)));
    let t2 = Arc::new(Mutex::new(Counter2(0)));

    let t1_for_w1 = Arc::clone(&t1);
    let t2_for_w1 = Arc::clone(&t2);
    let w1 = thread::spawn(move || {
        // Acquire locks in the global order: t1 first, then t2.
        let mut g1 = t1_for_w1.lock().unwrap();
        let _g2 = t2_for_w1.lock().unwrap();
        // Critical work performed while holding both locks.
        g1.0 = 1;
        // Guards drop here, releasing both locks.
    });

    let t1_for_w2 = Arc::clone(&t1);
    let t2_for_w2 = Arc::clone(&t2);
    let w2 = thread::spawn(move || {
        // Same global order: t1 first, then t2. Identical ordering in
        // every worker makes the ABBA deadlock impossible.
        let _g1 = t1_for_w2.lock().unwrap();
        let mut g2 = t2_for_w2.lock().unwrap();
        // Critical work performed while holding both locks.
        g2.0 = 1;
        // Guards drop here, releasing both locks.
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let v1 = t1.lock().unwrap().0;
    let v2 = t2.lock().unwrap().0;
    println!("DONE t1={} t2={}", v1, v2);
}
