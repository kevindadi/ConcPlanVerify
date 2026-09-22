mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// Sequential helper routine: performs only local computation.
/// It touches no shared state.
fn sequential_helper() -> u64 {
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i.wrapping_mul(i));
    }
    acc
}

fn main() { cir_trace::init();
    // Shared counter, protected by the shared mutex.
    let shared = Arc::new(Mutex::new_named("shared_mutex0", 0u32));

    // R1: the main task starts a group of two workers.
    let mut workers = Vec::new();
    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        workers.push(thread::spawn(move || {
            // R5: lock() waits until a held mutex becomes free.
            // R4: the mutex guarantees at most one holder at any moment.
            let mut guard = shared.lock().unwrap();

            // R2: while holding the mutex, run the purely local helper,
            // then update the shared counter.
            let _local_result = sequential_helper();
            *guard = 1;

            // R3: release the mutex before the worker finishes.
            drop(guard);
        }));
    }

    // R7: both workers complete (main joins each one).
    for w in workers {
        w.join().unwrap();
    }

    // R8: print exactly the required line, then exit.
    let done = *shared.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
