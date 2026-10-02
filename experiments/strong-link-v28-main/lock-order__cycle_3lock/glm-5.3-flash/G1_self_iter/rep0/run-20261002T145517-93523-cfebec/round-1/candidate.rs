use std::sync::{Arc, Mutex};
use std::thread;

// Global lock order: a < b < c. Every worker acquires its locks in
// ascending order of this ranking, which makes circular wait impossible.

fn main() {
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));
    let c = Arc::new(Mutex::new(0u64));

    // t1 needs a and b: acquire a, then b (ascending order).
    let t1 = {
        let (a, b) = (Arc::clone(&a), Arc::clone(&b));
        thread::spawn(move || {
            let mut ga = a.lock().unwrap();
            let mut gb = b.lock().unwrap();
            // Critical work: both locks held simultaneously.
            *ga += 1;
            *gb += 1;
            drop(gb);
            drop(ga);
        })
    };

    // t2 needs b and c: acquire b, then c (ascending order).
    let t2 = {
        let (b, c) = (Arc::clone(&b), Arc::clone(&c));
        thread::spawn(move || {
            let mut gb = b.lock().unwrap();
            let mut gc = c.lock().unwrap();
            // Critical work: both locks held simultaneously.
            *gb += 1;
            *gc += 1;
            drop(gc);
            drop(gb);
        })
    };

    // t3 needs c and a: acquire a, then c (ascending order — this is the
    // deadlock fix; the naive c-then-a order creates a circular wait).
    let t3 = {
        let (a, c) = (Arc::clone(&a), Arc::clone(&c));
        thread::spawn(move || {
            let mut ga = a.lock().unwrap();
            let mut gc = c.lock().unwrap();
            // Critical work: both locks held simultaneously.
            *ga += 1;
            *gc += 1;
            drop(gc);
            drop(ga);
        })
    };

    // Main starts all three workers and finishes only after all three finish.
    for handle in [t1, t2, t3] {
        handle.join().expect("worker thread panicked");
    }

    println!("DONE done=1");
}
