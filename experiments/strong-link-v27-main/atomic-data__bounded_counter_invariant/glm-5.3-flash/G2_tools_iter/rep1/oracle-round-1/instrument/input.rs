use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: m is the mutual-exclusion lock guarding c.
    // c is the shared counter, declared to range from 0 to 2, starting at 0.
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

    let m_for_w1 = Arc::clone(&m);
    let m_for_w2 = Arc::clone(&m);

    // Supervising task (main) launches the two worker threads w1 and w2.
    let w1 = thread::spawn(move || {
        // Hold the lock for the entire read-modify-write of the shared counter.
        let mut c = m_for_w1.lock().unwrap();
        *c += 1;
        // Lock released when c goes out of scope.
    });

    let w2 = thread::spawn(move || {
        // Hold the lock for the entire read-modify-write of the shared counter.
        let mut c = m_for_w2.lock().unwrap();
        *c += 1;
        // Lock released when c goes out of scope.
    });

    // Wait for both workers to finish.
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Print exactly the required line, then exit.
    println!("DONE done=1");
}
