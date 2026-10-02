use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);
    let handle = thread::spawn(move || outer(outer_a, outer_b));

    handle.join().unwrap();
    println!("DONE done=1");
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x2_a = Arc::clone(&a);
    let x2_b = Arc::clone(&b);

    let handle1 = thread::spawn(move || x1(x1_a, x1_b));
    let handle2 = thread::spawn(move || x2(x2_a, x2_b));

    handle1.join().unwrap();
    handle2.join().unwrap();
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}
