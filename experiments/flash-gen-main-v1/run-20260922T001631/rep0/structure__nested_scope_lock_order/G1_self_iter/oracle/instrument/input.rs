use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // R1: main task starts one outer worker.
    let outer = thread::spawn(|| {
        let a = Arc::new(Mutex::new(0));
        let b = Arc::new(Mutex::new(0));

        // R2: outer worker starts a nested group of two inner tasks.
        let mut handles = Vec::new();
        for _ in 0..2 {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            handles.push(thread::spawn(move || {
                // R3 & R4: take A then B, same order in both tasks.
                let mut ga = a.lock().unwrap();
                let mut gb = b.lock().unwrap();
                *ga += 1;
                *gb += 1;
            }));
        }

        // R5: outer worker completes only after both inner tasks finish.
        for h in handles {
            h.join().unwrap();
        }
    });

    outer.join().unwrap();

    // R7: print exactly the required line.
    println!("DONE done=1");
}
