use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // c: the shared counter. Declared range is 0..=2 (u8 is more than wide
    // enough), and it starts at zero (R2).
    // m: the mutual-exclusion lock guarding c (R4). Arc<Mutex<u8>> gives us
    // both the lock and shared ownership across threads.
    let c: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));
    let m = Arc::clone(&c); // m is the lock protecting c

    // Supervising task (R1): main launches w1 and w2 and joins both.

    // Worker w1
    let m_w1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        // Hold the lock for the entire read-modify-write (R4).
        let mut guard = m_w1.lock().unwrap();
        // Increment only if the result stays within the required upper
        // limit. R5 demands c <= 1 in every reachable state, so the
        // effective upper bound for an increment is 1: incrementing when
        // c == 1 would produce 2 and violate R5, even though the declared
        // range allows it (R3).
        if *guard < 1 {
            *guard += 1;
        }
        // Lock released here, at end of scope.
    });

    // Worker w2
    let m_w2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        let mut guard = m_w2.lock().unwrap();
        if *guard < 1 {
            *guard += 1;
        }
    });

    // Wait for both workers (R1, R6). join() blocks until each thread
    // terminates; there are no loops or condition variables, so every
    // interleaving terminates (R6).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Exactly one worker finds c == 0 and increments; the other observes
    // c == 1 under the lock and does nothing. Hence c == 1 here (R5, R7).
    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
}
