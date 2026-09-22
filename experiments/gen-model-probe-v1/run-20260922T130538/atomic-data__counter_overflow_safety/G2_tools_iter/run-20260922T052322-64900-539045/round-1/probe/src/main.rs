use std::sync::{Arc, Mutex};
use std::thread;

/// State shared by the two worker threads.
struct Shared {
    /// Declared range: 0..=2 (inclusive). Starts at 0.
    counter: u32,
    /// Becomes true once one worker has performed its increment,
    /// which keeps the reachable maximum of `counter` at 1.
    claimed: bool,
}

fn main() {
    // Supervising task: create the shared state, launch two workers,
    // then wait for both to finish.
    let shared = Arc::new(Mutex::new(Shared {
        counter: 0,
        claimed: false,
    }));

    let mut workers = Vec::new();
    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        workers.push(thread::spawn(move || {
            // Hold the mutual-exclusion lock for the entire
            // read-modify-write of the shared counter.
            let mut state = shared.lock().unwrap();
            // Add one only while the result stays within the declared
            // upper limit of 2, and only if no worker has claimed the
            // single available increment yet.
            if !state.claimed && state.counter < 2 {
                state.counter += 1;
                state.claimed = true;
            }
            // Guard drops here, releasing the lock.
        }));
    }

    // Wait for both workers to finish; each worker terminates after a
    // single bounded critical section, so every interleaving terminates.
    for worker in workers {
        worker.join().unwrap();
    }

    // Read the final counter under the lock as well.
    let state = shared.lock().unwrap();
    println!("DONE done={}", state.counter);
}
