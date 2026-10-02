use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_handle = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || x1(a, b)
    });

    let x2_handle = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || x2(a, b)
    });

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
