use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    let mut x = 0;
    for i in 0..1000 {
        x = x.wrapping_add(i);
    }
    x
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let acc = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = thread::spawn(move || {
        let _guard = m1.lock().unwrap();
        let _ = compute();
        let mut a = acc1.lock().unwrap();
        *a += 1;
    });

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        let _ = compute();
        let mut a = acc2.lock().unwrap();
        *a += 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
}
