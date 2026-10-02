use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    drop(_guard_b);
    drop(_guard_a);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    drop(_guard_b);
    drop(_guard_a);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || x1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || x2(a, b))
    };
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let outer_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || outer(a, b))
    };
    outer_handle.join().unwrap();

    println!("DONE done=1");
}
