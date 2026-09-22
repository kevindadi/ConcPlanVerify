use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a (lock), b (lock).
    let a = Arc::new(Mutex::new(0u64));
    let b = Arc::new(Mutex::new(0u64));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    // R1: main starts one outer worker.
    let outer = thread::spawn(move || {
        // R2: outer starts a nested group of two inner tasks, x1 and x2.

        let a_x1 = Arc::clone(&a_outer);
        let b_x1 = Arc::clone(&b_outer);
        let x1 = thread::spawn(move || {
            // R3/R4: take a then b; both are held simultaneously.
            let mut ga = a_x1.lock().unwrap();
            let mut gb = b_x1.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        let a_x2 = Arc::clone(&a_outer);
        let b_x2 = Arc::clone(&b_outer);
        let x2 = thread::spawn(move || {
            // R3/R4: same lock order (a then b) — no wait cycle can form.
            let mut ga = a_x2.lock().unwrap();
            let mut gb = b_x2.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        // R5: outer completes only after both inner tasks have finished.
        x1.join().unwrap();
        x2.join().unwrap();
    });

    outer.join().unwrap();

    // R7: print exactly this line and exit.
    println!("DONE done=1");
}
