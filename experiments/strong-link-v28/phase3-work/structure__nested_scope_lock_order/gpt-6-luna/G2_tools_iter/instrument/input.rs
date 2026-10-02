use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x2_a = Arc::clone(&a);
    let x2_b = Arc::clone(&b);

    let task_x1 = thread::spawn(move || x1(x1_a, x1_b));
    let task_x2 = thread::spawn(move || x2(x2_a, x2_b));

    task_x1.join().unwrap();
    task_x2.join().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let worker = thread::spawn(move || outer(a, b));
    worker.join().unwrap();

    println!("DONE done=1");
}
