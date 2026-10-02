use std::sync::{Arc, Mutex};
use std::thread;

fn compute(seed: i32) -> i32 {
    let mut tmp = seed;
    tmp = tmp + 1;
    tmp
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(1);
    *acc = 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    let _ = compute(1);
    *acc = 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = thread::spawn(move || w1(m1));
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
