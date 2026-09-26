use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
}

fn main() {
    let _semaphore = Semaphore::new(4);

    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));
    let done = Arc::new(AtomicUsize::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = thread::spawn(move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = thread::spawn(move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    done.store(1, Ordering::SeqCst);
    println!("DONE done={}", done.load(Ordering::SeqCst));
}
