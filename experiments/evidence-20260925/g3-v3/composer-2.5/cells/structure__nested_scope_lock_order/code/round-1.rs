use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    let _lock_a = a.lock().unwrap();
    let lock_b = b.lock().unwrap();
    drop(lock_b);
    drop(_lock_a);
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    let _lock_a = a.lock().unwrap();
    let lock_b = b.lock().unwrap();
    drop(lock_b);
    drop(_lock_a);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    thread::scope(|s| {
        let a1 = Arc::clone(&a);
        let b1 = Arc::clone(&b);
        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        s.spawn(move || x1(&a1, &b1)).join().unwrap();
        s.spawn(move || x2(&a2, &b2)).join().unwrap();
    });
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    thread::scope(|s| {
        let a_outer = Arc::clone(&a);
        let b_outer = Arc::clone(&b);
        s.spawn(move || outer(a_outer, b_outer)).join().unwrap();
    });

    println!("DONE done=1");
}
