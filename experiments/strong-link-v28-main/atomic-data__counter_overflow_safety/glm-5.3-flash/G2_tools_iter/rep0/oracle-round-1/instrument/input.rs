use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // m: the mutual-exclusion lock.
    // c: the shared counter, declared range 0..=2, initial value 0.
    // (The i32 guarded by m is c.)
    let m = Arc::new(Mutex::new(0i32));

    // Worker w1
    let m_w1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        // Hold the lock m for the whole read-modify-write of c.
        let mut c = m_w1.lock().unwrap();
        // Only add one if the result stays within the required upper limit (1).
        if *c < 1 {
            *c += 1;
        }
        // Lock released when `c` (the guard) goes out of scope.
    });

    // Worker w2
    let m_w2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        // Hold the lock m for the whole read-modify-write of c.
        let mut c = m_w2.lock().unwrap();
        // Only add one if the result stays within the required upper limit (1).
        if *c < 1 {
            *c += 1;
        }
        // Lock released when `c` (the guard) goes out of scope.
    });

    // Supervisor waits for both workers to finish (every schedule terminates:
    // neither worker ever blocks on the other, since the lock is never held
    // across a wait).
    w1.join().unwrap();
    w2.join().unwrap();

    // Exactly one worker performed its increment, so c == 1 here.
    let c = m.lock().unwrap();
    println!("DONE done={}", c);
}
