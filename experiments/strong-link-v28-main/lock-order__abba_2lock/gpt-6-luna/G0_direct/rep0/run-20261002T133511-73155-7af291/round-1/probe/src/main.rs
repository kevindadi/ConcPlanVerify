use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
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

    let t1_result = t1_handle.join().unwrap();
    let t2_result = t2_handle.join().unwrap();

    println!("DONE t1={} t2={}", t1_result, t2_result);
}
