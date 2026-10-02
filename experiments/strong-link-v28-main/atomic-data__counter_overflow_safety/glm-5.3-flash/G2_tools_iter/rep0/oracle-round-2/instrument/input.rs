use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // m: the mutual-exclusion lock.
    // c: the shared counter guarded by m; declared range 0..=2, initial value 0.
    let m: Arc<Mutex<u8>> = Arc::new(Mutex::new(0));

    // Worker w1: increment c under m, but only if the result stays within
    // the required upper limit (1).
    let w1 = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            let mut c = m.lock().unwrap(); // acquire m
            if *c < 1 {
                *c += 1;
            }
            drop(c); // release m immediately; never held across a wait or I/O
        })
    };

    // Worker w2: identical behaviour to w1.
    let w2 = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            let mut c = m.lock().unwrap(); // acquire m
            if *c < 1 {
                *c += 1;
            }
            drop(c); // release m immediately
        })
    };

    // Supervisor: wait for both workers to finish. No lock is held on this
    // thread while waiting, and neither worker ever blocks on the other
    // (the lock is never held across a wait), so every interleaving terminates.
    w1.join().unwrap();
    w2.join().unwrap();

    // Exactly one worker performed its increment, so c == 1 here.
    // Read the final value of c while holding m, then release m before
    // doing any I/O.
    let done = {
        let c = m.lock().unwrap();
        *c
    };

    println!("DONE done={}", done);
}
