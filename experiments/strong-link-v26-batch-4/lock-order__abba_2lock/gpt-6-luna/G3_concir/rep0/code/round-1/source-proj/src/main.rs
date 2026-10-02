use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(_a_guard);
    let _ = work;
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(_a_guard);
    let _ = work;
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let t1_a = Arc::clone(&a);
    let t1_b = Arc::clone(&b);
    let worker1 = thread::spawn(move || t1(t1_a, t1_b));

    let t2_a = Arc::clone(&a);
    let t2_b = Arc::clone(&b);
    let worker2 = thread::spawn(move || t2(t2_a, t2_b));

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
