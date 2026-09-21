mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn sequential_helper() -> u64 {
    // Performs only local computation.
    let mut acc: u64 = 0;
    for i in 0..1000u64 {
        acc = acc.wrapping_add(i.wrapping_mul(3).wrapping_add(1));
    }
    acc
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", 0u64));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        let handle = cir_trace::spawn("handle", move || {
            // R2: take the shared mutex.
            let mut guard = shared.lock().unwrap();

            // R2: call sequential helper (local computation) while holding mutex.
            let local = sequential_helper();

            // R2: update shared counter while still holding the mutex.
            // Use local result so the computation is not optimized away.
            *guard = guard.wrapping_add(local % 2);

            // R3: release the mutex before finishing.
            drop(guard);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // R8: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
