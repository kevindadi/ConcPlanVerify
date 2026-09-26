use std::sync::{Arc, Mutex};
use std::thread;

fn compute(x: i32) -> i32 {
    let y = x;
    y
}

fn w1(m: &Mutex<i32>) {
    let mut guard = m.lock().unwrap();
    let _ = compute(1);
    *guard = 1;
}

fn w2(m: &Mutex<i32>) {
    let mut guard = m.lock().unwrap();
    let _ = compute(1);
    *guard = 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || {
        w2(&m2);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
}
