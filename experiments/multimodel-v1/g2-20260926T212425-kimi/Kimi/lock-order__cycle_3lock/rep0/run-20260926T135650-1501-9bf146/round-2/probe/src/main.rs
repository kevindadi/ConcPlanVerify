use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Three shared locks.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // Worker t1 needs locks a and b.
    // Acquire in global order a -> b.
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = thread::spawn(move || {
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Critical work while holding both a and b.
        // Guards drop here, releasing both locks.
    });

    // Worker t2 needs locks b and c.
    // Acquire in global order b -> c.
    let (b2, c2) = (Arc::clone(&b), Arc::clone(&c));
    let t2 = thread::spawn(move || {
        let _guard_b = b2.lock().unwrap();
        let _guard_c = c2.lock().unwrap();
        // Critical work while holding both b and c.
    });

    // Worker t3 needs locks c and a.
    // Acquire in global order a -> c (NOT c -> a) so that all workers
    // request locks in a consistent global order a < b < c. This makes
    // circular waiting impossible, so no deadlock can occur in any
    // interleaving.
    let (c3, a3) = (Arc::clone(&c), Arc::clone(&a));
    let t3 = thread::spawn(move || {
        let _guard_a = a3.lock().unwrap();
        let _guard_c = c3.lock().unwrap();
        // Critical work while holding both a and c.
    });

    // Main thread waits for all workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
