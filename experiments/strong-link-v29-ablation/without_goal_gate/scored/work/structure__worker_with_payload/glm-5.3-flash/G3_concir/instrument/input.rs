use std::sync::{Arc, Mutex};
use std::thread;

// Shared resource: m is the mutex; acc is the variable it protects,
// stored as a primitive inside the mutex.
fn compute(n: i32) -> i32 {
    let mut r: i32 = 0;
    r = n + 1;
    r = r * 2;
    r
}

fn w1(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute(1);
    *guard = 1;
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute(1);
    *guard = 1;
    drop(guard);
}

fn main() {
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(&m1));
    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || w2(&m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
}
