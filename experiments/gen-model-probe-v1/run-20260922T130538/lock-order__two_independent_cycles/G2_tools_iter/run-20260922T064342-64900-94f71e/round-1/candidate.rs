use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Four shared locks arranged as two independent pairs.
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));
    let c = Arc::new(Mutex::new(0u64));
    let d = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();

    // First pair: both workers acquire `a` then `b` (same relative order),
    // hold both while working, and release them via RAII guards.
    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let mut ga = a.lock().unwrap();
            let mut gb = b.lock().unwrap();
            *ga += 1;
            *gb += 1;
        }));
    }

    // Second pair: both workers acquire `c` then `d` (same relative order),
    // independent of the first pair.
    for _ in 0..2 {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let mut gc = c.lock().unwrap();
            let mut gd = d.lock().unwrap();
            *gc += 1;
            *gd += 1;
        }));
    }

    // Main finishes only after all four workers have finished.
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
