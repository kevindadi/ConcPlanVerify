use std::sync::{Arc, Mutex};
use std::thread;

fn sequential_helper() -> u64 {
    // Performs only local computation.
    let mut acc: u64 = 0;
    for i in 0..1000u64 {
        acc = acc.wrapping_add(i.wrapping_mul(3).wrapping_add(1));
    }
    acc
}

fn main() {
    let shared = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        let handle = thread::spawn(move || {
            // R2: take the shared mutex.
            let mut guard = shared.lock().unwrap();

            // R2: call sequential helper while holding the mutex.
            let _local = sequential_helper();

            // R2: update shared counter while still holding the mutex.
            *guard += 1;

            // R3: release the mutex before finishing.
            drop(guard);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let done = *shared.lock().unwrap();
    println!("DONE done={}", done);
}
