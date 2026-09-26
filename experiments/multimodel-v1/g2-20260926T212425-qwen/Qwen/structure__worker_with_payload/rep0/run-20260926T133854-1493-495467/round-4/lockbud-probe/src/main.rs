use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    // Sequential helper routine that performs only local computation.
    1
}

fn main() {
    let acc = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&acc);
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        let val = compute();
        *guard += val;
        drop(guard);
    });

    let m2 = Arc::clone(&acc);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        let val = compute();
        *guard += val;
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done={}", *acc.lock().unwrap());
}
