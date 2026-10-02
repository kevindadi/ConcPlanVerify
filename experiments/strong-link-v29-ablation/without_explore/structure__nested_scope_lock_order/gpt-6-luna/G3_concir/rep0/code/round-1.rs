use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_handle = thread::spawn(move || x1(a.clone(), b.clone()));
    let x2_handle = thread::spawn(move || x2(a, b));

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let outer_handle = thread::spawn(move || outer(a, b));
    outer_handle.join().unwrap();

    println!("DONE done=1");
}
