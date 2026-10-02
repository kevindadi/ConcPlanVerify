use std::sync::{Arc, Mutex};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let w1_a = Arc::clone(&a);
    let w1_b = Arc::clone(&b);
    let handle1 = thread::spawn(move || w1(w1_a, w1_b));

    let w2_a = Arc::clone(&a);
    let w2_b = Arc::clone(&b);
    let handle2 = thread::spawn(move || w2(w2_a, w2_b));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
}
