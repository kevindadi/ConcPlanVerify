use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));

    // R1: main starts one outer worker.
    let outer = thread::spawn(move || {
        let a1 = Arc::clone(&a);
        let b1 = Arc::clone(&b);
        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);

        // R2: nested group of two inner tasks.
        let t1 = thread::spawn(move || {
            // R3 + R4: lock A then B, holding both simultaneously.
            let mut ga = a1.lock().unwrap();
            let mut gb = b1.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });
        let t2 = thread::spawn(move || {
            // R3 + R4: same order (A then B), so no wait cycle can form.
            let mut ga = a2.lock().unwrap();
            let mut gb = b2.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        // R5: outer completes only after both inner tasks finish.
        t1.join().unwrap();
        t2.join().unwrap();
    });

    outer.join().unwrap();

    // R7: exact output line.
    println!("DONE done=1");
}
