use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    acc: i32,
}

fn compute() {
    let x: i32 = 1;
    let _y: i32 = x + 1;
}

fn w1(shared: Arc<Mutex<Shared>>) {
    let mut guard = shared.lock().unwrap();
    compute();
    let tmp = guard.acc;
    let tmp2 = tmp + 1;
    guard.acc = tmp2;
    drop(guard);
}

fn w2(shared: Arc<Mutex<Shared>>) {
    let mut guard = shared.lock().unwrap();
    compute();
    let tmp = guard.acc;
    let tmp2 = tmp + 1;
    guard.acc = tmp2;
    drop(guard);
}

fn main() {
    let shared = Arc::new(Mutex::new(Shared { acc: 0 }));

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = shared.lock().unwrap().acc;
    println!("DONE done={}", done);
}
