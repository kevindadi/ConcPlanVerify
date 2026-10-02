use std::sync::{Arc, Mutex};
use std::thread;

fn critical_work(a: &Mutex<()>, b: &Mutex<()>) -> usize {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    // Both locks are held while the critical work is performed.
    let result = 1;

    drop(b_guard);
    drop(a_guard);
    result
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let worker1 = thread::spawn(move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let worker2 = thread::spawn(move || t2(a2, b2));

    let result1 = worker1.join().unwrap();
    let result2 = worker2.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
}
