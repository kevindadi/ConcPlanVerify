use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared counter c (declared range 0..=2, starts at 0),
    // guarded by the mutual-exclusion lock m.
    let m = Arc::new(Mutex::new(0i32));

    // Worker w1: adds exactly one to c while holding m.
    let m1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        let mut c = m1.lock().unwrap();
        *c += 1;
    });

    // Worker w2: adds exactly one to c while holding m.
    let m2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        let mut c = m2.lock().unwrap();
        *c += 1;
    });

    // Supervising task waits for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
