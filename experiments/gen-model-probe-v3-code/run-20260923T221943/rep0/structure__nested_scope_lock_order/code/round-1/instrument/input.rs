use std::sync::{Arc, Mutex};
use std::thread;

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

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || x1(a1, b1));
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || x2(a2, b2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let mut done = 0;
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let a_cl = Arc::clone(&a);
    let b_cl = Arc::clone(&b);
    let h = thread::spawn(move || outer(a_cl, b_cl));
    h.join().unwrap();
    done = 1;
    println!("DONE done={}", done);
}
