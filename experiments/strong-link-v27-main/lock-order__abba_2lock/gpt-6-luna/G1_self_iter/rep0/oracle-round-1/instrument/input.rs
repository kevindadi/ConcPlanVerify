use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let result = 1; // Critical work while holding both locks.

    drop(b_guard);
    drop(a_guard);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let result = 1; // Critical work while holding both locks.

    drop(b_guard);
    drop(a_guard);
    result
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let handle_t1 = thread::spawn(move || t1(a_for_t1, b_for_t1));

    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);
    let handle_t2 = thread::spawn(move || t2(a_for_t2, b_for_t2));

    let result_t1 = handle_t1.join().unwrap();
    let result_t2 = handle_t2.join().unwrap();

    println!("DONE t1={} t2={}", result_t1, result_t2);
}
