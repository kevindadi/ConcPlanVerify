use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, count: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    count.fetch_add(1, Ordering::SeqCst);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, count: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    count.fetch_add(1, Ordering::SeqCst);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let t1_count = Arc::new(AtomicUsize::new(0));
    let t2_count = Arc::new(AtomicUsize::new(0));

    let worker_t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let count = Arc::clone(&t1_count);
        thread::spawn(move || t1(a, b, count))
    };

    let worker_t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let count = Arc::clone(&t2_count);
        thread::spawn(move || t2(a, b, count))
    };

    worker_t1.join().unwrap();
    worker_t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_count.load(Ordering::SeqCst),
        t2_count.load(Ordering::SeqCst)
    );
}
