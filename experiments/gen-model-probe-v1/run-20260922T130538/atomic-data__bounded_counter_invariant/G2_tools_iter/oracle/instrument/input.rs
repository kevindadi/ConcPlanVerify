use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared counter, declared range 0..=2, starts at 0.
    let counter = Arc::new(Mutex::new(0i32));

    let mut handles = Vec::new();

    // Supervising task launches two worker threads.
    for _ in 0..2 {
        let shared = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            // Hold the mutual-exclusion lock while reading/writing the counter.
            let mut guard = shared.lock().unwrap();
            // Each worker adds exactly one. Since both workers do this under
            // the lock, the counter only ever takes values 0, 1, and 2.
            *guard += 1;
        }));
    }

    // Wait for both workers to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
}
