use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    let payload = 1;
    let result = payload;
    result
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute();
    *guard = *guard + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _ = compute();
    *guard = *guard + 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
