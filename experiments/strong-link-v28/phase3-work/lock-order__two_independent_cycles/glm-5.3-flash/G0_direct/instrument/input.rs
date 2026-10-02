use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a, b (first pair); c, d (second pair)
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let mut handles = Vec::new();

    // t1 and t2: both take a then b, in the same relative sequence (R6)
    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _ga = a.lock().unwrap(); // take a
            let _gb = b.lock().unwrap(); // take b — holds both (R4)
            // ... work while holding both locks of the first pair ...
            drop(_gb); // release b (R9)
            drop(_ga); // release a (R9)
        }));
    }

    // t3 and t4: both take c then d, in the same relative sequence (R7)
    for _ in 0..2 {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _gc = c.lock().unwrap(); // take c
            let _gd = d.lock().unwrap(); // take d — holds both (R4)
            // ... work while holding both locks of the second pair ...
            drop(_gd); // release d (R9)
            drop(_gc); // release c (R9)
        }));
    }

    // Main thread starts all four workers above and waits for all (R10)
    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
