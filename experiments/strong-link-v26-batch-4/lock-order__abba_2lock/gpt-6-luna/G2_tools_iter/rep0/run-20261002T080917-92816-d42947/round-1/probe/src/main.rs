use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> u8 {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let result = 1;

    drop(b_guard);
    drop(a_guard);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> u8 {
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
    let worker_t1 = thread::Builder::new()
        .name("t1".to_string())
        .spawn(move || t1(a1, b1))
        .unwrap();

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let worker_t2 = thread::Builder::new()
        .name("t2".to_string())
        .spawn(move || t2(a2, b2))
        .unwrap();

    let t1_result = worker_t1.join().unwrap();
    let t2_result = worker_t2.join().unwrap();

    println!("DONE t1={} t2={}", t1_result, t2_result);
}
