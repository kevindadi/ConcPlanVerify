use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x: i64 = 21 * 2;
    let y: i64 = x + 1;
    let _ = y;
}

fn w1(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    compute();
    let mut t = *acc;
    t += 1;
    *acc = t;
    drop(acc);
}

fn w2(m: Arc<Mutex<i64>>) {
    let mut acc = m.lock().unwrap();
    compute();
    let mut t = *acc;
    t += 1;
    *acc = t;
    drop(acc);
}

fn main() {
    let m = Arc::new(Mutex::new(0i64));

    let h1 = thread::spawn({
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let h2 = thread::spawn({
        let m = Arc::clone(&m);
        move || w2(m)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
