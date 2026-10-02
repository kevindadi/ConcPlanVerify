use std::sync::{Arc, Mutex};
use std::thread;

type Lock = Arc<Mutex<()>>;

fn t1(a: Lock, b: Lock) {
    let first = a.lock().unwrap();
    let second = b.lock().unwrap();

    std::hint::black_box((&*first, &*second));

    drop(second);
    drop(first);
}

fn t2(b: Lock, c: Lock) {
    let first = b.lock().unwrap();
    let second = c.lock().unwrap();

    std::hint::black_box((&*first, &*second));

    drop(second);
    drop(first);
}

fn t3(a: Lock, c: Lock) {
    // Acquire locks in the same global order as the other workers: a, b, c.
    let first = a.lock().unwrap();
    let second = c.lock().unwrap();

    std::hint::black_box((&*first, &*second));

    drop(second);
    drop(first);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };

    let t2_handle = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        thread::spawn(move || t2(b, c))
    };

    let t3_handle = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        thread::spawn(move || t3(a, c))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();

    println!("DONE done=1");
}
