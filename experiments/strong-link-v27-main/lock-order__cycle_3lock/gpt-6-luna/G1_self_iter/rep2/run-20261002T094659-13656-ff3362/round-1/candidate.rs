use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();

    // Critical work is performed while both locks are held.
    std::hint::black_box(());
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let _b = b.lock().unwrap();
    let _c = c.lock().unwrap();

    std::hint::black_box(());
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _c = c.lock().unwrap();

    std::hint::black_box(());
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let worker1 = {
        let (a, b) = (Arc::clone(&a), Arc::clone(&b));
        thread::spawn(move || t1(a, b))
    };
    let worker2 = {
        let (b, c) = (Arc::clone(&b), Arc::clone(&c));
        thread::spawn(move || t2(b, c))
    };
    let worker3 = {
        let (a, c) = (Arc::clone(&a), Arc::clone(&c));
        thread::spawn(move || t3(a, c))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();
    worker3.join().unwrap();

    println!("DONE done=1");
}
