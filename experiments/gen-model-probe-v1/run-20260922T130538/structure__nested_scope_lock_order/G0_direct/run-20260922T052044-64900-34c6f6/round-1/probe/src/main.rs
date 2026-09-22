use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // R1: main task starts one outer worker.
    let mutex_a = Arc::new(Mutex::new(0u64));
    let mutex_b = Arc::new(Mutex::new(0u64));

    let outer = thread::spawn({
        let mutex_a = Arc::clone(&mutex_a);
        let mutex_b = Arc::clone(&mutex_b);
        move || {
            // R2: the outer worker starts a nested group of two inner tasks.
            let mut inner_handles = Vec::new();
            for _ in 0..2 {
                let a = Arc::clone(&mutex_a);
                let b = Arc::clone(&mutex_b);
                inner_handles.push(thread::spawn(move || {
                    // R4: both inner tasks acquire the mutexes in the same
                    // order (A before B), so no wait cycle can form and every
                    // interleaving terminates (R6).
                    let mut guard_a = a.lock().unwrap();
                    let mut guard_b = b.lock().unwrap();
                    // R3: both mutexes are held at the same time here.
                    *guard_a += 1;
                    *guard_b += 1;
                }));
            }
            // R5: the outer worker completes only after both inner tasks
            // have finished.
            for handle in inner_handles {
                handle.join().unwrap();
            }
        }
    });

    outer.join().unwrap();

    // R7: print exactly this line and then exit.
    println!("DONE done=1");
}
