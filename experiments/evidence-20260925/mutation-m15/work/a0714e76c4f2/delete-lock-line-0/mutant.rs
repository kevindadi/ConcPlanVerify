use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x: i32 = 1;
    let y: i32 = x + 1;
    let _ = y;
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        compute();
        let mut tmp = *guard;
        tmp = tmp + 1;
        *guard = tmp;
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        compute();
        let mut tmp = *guard;
        tmp = tmp + 1;
        *guard = tmp;
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
}
