use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(a_guard);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(a_guard);
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

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
}
