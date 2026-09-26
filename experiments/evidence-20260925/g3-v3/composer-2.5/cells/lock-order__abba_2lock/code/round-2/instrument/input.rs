use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

type A = Arc<Mutex<()>>;

fn t1(a: A, b: A, t1_var: Arc<Mutex<i32>>, release_t2: Arc<AtomicBool>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    *t1_var.lock().unwrap() = 1;
    drop(gb);
    drop(ga);
    release_t2.store(true, Ordering::Release);
}

fn t2(a: A, b: A, t2_var: Arc<Mutex<i32>>, release_t2: Arc<AtomicBool>) {
    while !release_t2.load(Ordering::Acquire) {
        thread::yield_now();
    }
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    *t2_var.lock().unwrap() = 1;
    drop(gb);
    drop(ga);
}

fn main() {
    let a: A = Arc::new(Mutex::new(()));
    let b: A = Arc::new(Mutex::new(()));
    let t1_var = Arc::new(Mutex::new(0));
    let t2_var = Arc::new(Mutex::new(0));
    let release_t2 = Arc::new(AtomicBool::new(false));

    let h1 = {
        let (a, b, t1_var, release_t2) = (
            Arc::clone(&a),
            Arc::clone(&b),
            Arc::clone(&t1_var),
            Arc::clone(&release_t2),
        );
        thread::spawn(move || t1(a, b, t1_var, release_t2))
    };
    let h2 = {
        let (a, b, t2_var, release_t2) = (
            Arc::clone(&a),
            Arc::clone(&b),
            Arc::clone(&t2_var),
            Arc::clone(&release_t2),
        );
        thread::spawn(move || t2(a, b, t2_var, release_t2))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        *t1_var.lock().unwrap(),
        *t2_var.lock().unwrap()
    );
}
