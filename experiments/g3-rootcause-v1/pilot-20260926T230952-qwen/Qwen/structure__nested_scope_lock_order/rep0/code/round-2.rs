use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h1 = thread::spawn(move || x1(a.clone(), b.clone()));
    let h2 = thread::spawn(move || x2(a.clone(), b.clone()));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let h_outer = thread::spawn(move || outer(a, b));
    h_outer.join().unwrap();

    println!("DONE done=1");
}
