use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared counter, declared range 0..=2, starting at 0.
    let counter = Arc::new(Mutex::new(0i32));

    // Supervisor launches two worker threads.
    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            // Hold the mutual-exclusion lock for the whole read-modify-write.
            let mut guard = counter.lock().unwrap();
            // The counter must stay within its declared range 0..=2.
            assert!(*guard < 2, "counter would exceed its declared range");
            *guard += 1;
        }));
    }

    // Supervisor waits for both workers to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
}
