use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    work.fetch_add(1, Ordering::Relaxed);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _b_guard = b.lock().unwrap();
    let _c_guard = c.lock().unwrap();
    work.fetch_add(1, Ordering::Relaxed);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _c_guard = c.lock().unwrap();
    work.fetch_add(1, Ordering::Relaxed);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let work = Arc::new(AtomicUsize::new(0));

    let h1 = {
        let (a, b, work) = (Arc::clone(&a), Arc::clone(&b), Arc::clone(&work));
        thread::spawn(move || t1(a, b, work))
    };
    let h2 = {
        let (b, c, work) = (Arc::clone(&b), Arc::clone(&c), Arc::clone(&work));
        thread::spawn(move || t2(b, c, work))
    };
    let h3 = {
        let (a, c, work) = (Arc::clone(&a), Arc::clone(&c), Arc::clone(&work));
        thread::spawn(move || t3(a, c, work))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    assert_eq!(work.load(Ordering::Relaxed), 3);
    println!("DONE done=1");
}
