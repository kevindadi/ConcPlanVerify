use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // R1: main starts one outer worker.
    let outer = thread::spawn(|| {
        // R2: outer worker starts a nested group of two inner tasks.
        let a = Arc::new(Mutex::new(0u32));
        let b = Arc::new(Mutex::new(0u32));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            handles.push(thread::spawn(move || {
                // R3 & R4: take A then B, holding both at once, same order.
                let mut ga = a.lock().unwrap();
                *ga += 1;
                let mut gb = b.lock().unwrap();
                *gb += 1;
                // both held here
                drop(gb);
                drop(ga);
            }));
        }

        // R5: outer completes only after both inner tasks finished.
        for h in handles {
            h.join().unwrap();
        }
    });

    outer.join().unwrap();

    // R7: print exactly this line and exit.
    println!("DONE done=1");
}
