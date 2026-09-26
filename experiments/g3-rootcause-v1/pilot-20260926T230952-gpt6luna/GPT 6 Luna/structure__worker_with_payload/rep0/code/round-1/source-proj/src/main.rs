use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let mut x = 0;
    x = x + 1;
    let _ = x;
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc = *acc + 1;
    drop(acc);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut acc = m.lock().unwrap();
    compute();
    *acc = *acc + 1;
    drop(acc);
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
