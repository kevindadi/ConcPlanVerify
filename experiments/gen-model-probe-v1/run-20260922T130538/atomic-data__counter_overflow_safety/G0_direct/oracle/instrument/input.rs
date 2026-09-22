use std::sync::{Arc, Mutex};
use std::thread;

/// Declared range of the shared counter: 0..=2.
const DECLARED_MAX: u32 = 2;
/// Required upper limit that must hold in every reachable state.
const UPPER_LIMIT: u32 = 1;

fn main() {
    // Supervising task: create the shared counter, starting at zero.
    let counter = Arc::new(Mutex::new(0u32));

    // Launch two worker threads.
    let mut workers = Vec::with_capacity(2);
    for _ in 0..2 {
        let shared = Arc::clone(&counter);
        workers.push(thread::spawn(move || {
            // Hold the mutual-exclusion lock for both the read and the write,
            // so the two workers never update the counter at the same time.
            let mut value = shared.lock().unwrap();
            // Add one only when doing so keeps the value within the
            // required upper limit.
            if *value + 1 <= UPPER_LIMIT {
                *value += 1;
            }
            // The declared range is never violated either.
            debug_assert!(*value <= DECLARED_MAX);
        }));
    }

    // Wait for both workers to finish.
    for worker in workers {
        worker.join().unwrap();
    }

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
}
