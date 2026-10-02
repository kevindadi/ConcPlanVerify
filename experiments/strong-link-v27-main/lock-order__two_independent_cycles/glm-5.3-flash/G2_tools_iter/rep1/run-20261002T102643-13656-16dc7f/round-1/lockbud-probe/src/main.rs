use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared locks: first pair (a, b), second pair (c, d).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    // t1: needs both locks of the first pair (a then b).
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // Holds both locks of its pair while working.
        // Locks are released automatically when guards drop at scope end.
    });

    // t2: needs both locks of the first pair, same relative sequence (a then b).
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // t3: needs both locks of the second pair (c then d).
    let c1 = Arc::clone(&c);
    let d1 = Arc::clone(&d);
    let t3 = thread::spawn(move || {
        let _gc = c1.lock().unwrap();
        let _gd = d1.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // t4: needs both locks of the second pair, same relative sequence (c then d).
    let c2 = Arc::clone(&c);
    let d2 = Arc::clone(&d);
    let t4 = thread::spawn(move || {
        let _gc = c2.lock().unwrap();
        let _gd = d2.lock().unwrap();
        // Holds both locks of its pair while working.
    });

    // Main thread starts all four workers and finishes only after all complete.
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
}
