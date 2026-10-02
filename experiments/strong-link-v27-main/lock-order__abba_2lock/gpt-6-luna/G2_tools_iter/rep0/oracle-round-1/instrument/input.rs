use std::sync::{Arc, Mutex};
use std::thread;

fn worker(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> u8 {
    // Both workers acquire the locks in the same order, preventing deadlock.
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let result = 1; // Critical work while holding both locks.

    drop(guard_b);
    drop(guard_a);
    result
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || worker(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || worker(a2, b2));

    let t1 = t1.join().unwrap();
    let t2 = t2.join().unwrap();

    println!("DONE t1={} t2={}", t1, t2);
}
