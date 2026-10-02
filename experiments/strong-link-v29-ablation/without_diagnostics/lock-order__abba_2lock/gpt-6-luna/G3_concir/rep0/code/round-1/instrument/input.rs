use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let result = 1;
    drop(b_guard);
    drop(a_guard);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let result = 1;
    drop(b_guard);
    drop(a_guard);
    result
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = thread::spawn(move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = thread::spawn(move || t2(a2, b2));

    let result1 = t1_handle.join().unwrap();
    let result2 = t2_handle.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
}
