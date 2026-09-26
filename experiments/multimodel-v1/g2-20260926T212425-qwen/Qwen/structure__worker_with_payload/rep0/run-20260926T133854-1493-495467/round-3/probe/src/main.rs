use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    // Sequential helper routine that performs only local computation.
    1
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        let val = compute();
        *guard += val;
    });

    let m2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        let val = compute();
        *guard += val;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done={}", *m.lock().unwrap());
}
