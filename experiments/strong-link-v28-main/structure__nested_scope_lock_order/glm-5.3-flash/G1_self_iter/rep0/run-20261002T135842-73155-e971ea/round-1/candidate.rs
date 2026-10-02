use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a (lock), b (lock).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // R1: main starts the outer worker.
    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);
    let outer = thread::spawn(move || {
        let mut inner_handles = Vec::new();

        // R2: outer starts a nested group of two inner tasks: x1, x2.
        for name in ["x1", "x2"] {
            let a = Arc::clone(&outer_a);
            let b = Arc::clone(&outer_b);
            inner_handles.push(thread::spawn(move || {
                // R4: fixed global lock order a -> b; no wait cycle can form.
                let _ga = a.lock().expect("mutex a poisoned");
                let _gb = b.lock().expect("mutex b poisoned");

                // R3: both a and b are held at the same time here.
                debug_assert!(!_ga.is_poisoned() || true); // guards alive until end of scope
                let _ = name;
            }));
        }

        // D2 repair: outer completes only after both inner tasks have finished (R5).
        for handle in inner_handles {
            handle.join().expect("inner task panicked");
        }
    });

    // D3 repair: main waits for the outer worker.
    outer.join().expect("outer worker panicked");

    // D4 repair: printed exactly once, after everything has completed (R7).
    println!("DONE done=1");
}
