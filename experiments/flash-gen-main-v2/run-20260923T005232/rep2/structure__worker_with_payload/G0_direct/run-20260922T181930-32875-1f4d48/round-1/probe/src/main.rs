use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    // Sequential helper routine performing only local computation.
    let mut x = 0;
    for i in 0..10 {
        x += i;
    }
    x
}

fn worker(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    let local = compute();
    let mut guard = m.lock().unwrap();
    let mut a = acc.lock().unwrap();
    *a += local;
    drop(a);
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let acc = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = thread::spawn(move || worker(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = thread::spawn(move || worker(m2, acc2));

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
}
