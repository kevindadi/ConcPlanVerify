use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

type A = Arc<Mutex<()>>;

fn t1(a: A, b: A, t1_var: Arc<AtomicI32>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    t1_var.store(1, Ordering::Relaxed);
}

fn t2(a: A, b: A, t2_var: Arc<AtomicI32>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    t2_var.store(1, Ordering::Relaxed);
}

fn main() {
    let a: A = Arc::new(Mutex::new(()));
    let b: A = Arc::new(Mutex::new(()));
    let t1_var = Arc::new(AtomicI32::new(0));
    let t2_var = Arc::new(AtomicI32::new(0));

    let h1 = {
        let (a, b, t1_var) = (Arc::clone(&a), Arc::clone(&b), Arc::clone(&t1_var));
        thread::spawn(move || t1(a, b, t1_var))
    };
    let h2 = {
        let (a, b, t2_var) = (Arc::clone(&a), Arc::clone(&b), Arc::clone(&t2_var));
        thread::spawn(move || t2(a, b, t2_var))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_var.load(Ordering::Relaxed),
        t2_var.load(Ordering::Relaxed)
    );
}
