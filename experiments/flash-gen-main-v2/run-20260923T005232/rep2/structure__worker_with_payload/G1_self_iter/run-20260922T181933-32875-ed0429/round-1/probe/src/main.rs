use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    // Sequential helper routine performing only local computation.
    let mut x: u64 = 0;
    for i in 0..1000 {
        x = x.wrapping_add(i);
    }
    std::hint::black_box(x);
}

fn main() {
    let m = Arc::new(Mutex::new(0u64));
    let acc = Arc::new(Mutex::new(0u64));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        compute();
        let mut a = acc1.lock().unwrap();
        *a += 1;
        *guard += 1;
        drop(a);
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        compute();
        let mut a = acc2.lock().unwrap();
        *a += 1;
        *guard += 1;
        drop(a);
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
}
