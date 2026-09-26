use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // First pair of locks, shared by t1 and t2.
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));
    // Second pair of locks, shared by t3 and t4 (independent of the first pair).
    let c = Arc::new(Mutex::new(0u64));
    let d = Arc::new(Mutex::new(0u64));

    // t1: acquires a, then b (consistent order within the first pair).
    let t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || {
            let mut la = a.lock().unwrap();
            let mut lb = b.lock().unwrap();
            // Work while holding both locks of the first pair.
            *la += 1;
            *lb += 1;
        })
    };

    // t2: acquires a, then b (same relative sequence as t1).
    let t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || {
            let mut la = a.lock().unwrap();
            let mut lb = b.lock().unwrap();
            // Work while holding both locks of the first pair.
            *la += 1;
            *lb += 1;
        })
    };

    // t3: acquires c, then d (consistent order within the second pair).
    let t3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || {
            let mut lc = c.lock().unwrap();
            let mut ld = d.lock().unwrap();
            // Work while holding both locks of the second pair.
            *lc += 1;
            *ld += 1;
        })
    };

    // t4: acquires c, then d (same relative sequence as t3).
    let t4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || {
            let mut lc = c.lock().unwrap();
            let mut ld = d.lock().unwrap();
            // Work while holding both locks of the second pair.
            *lc += 1;
            *ld += 1;
        })
    };

    // Main thread waits for all four workers to finish.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
}
