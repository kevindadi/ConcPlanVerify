use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    acc: i32,
}

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn worker(shared: Arc<Mutex<Shared>>) {
    let mut guard = shared.lock().unwrap();
    compute();
    let tmp = guard.acc;
    let tmp = tmp + 1;
    guard.acc = tmp;
    drop(guard);
}

fn main() {
    let shared = Arc::new(Mutex::new(Shared { acc: 0 }));

    let s1 = Arc::clone(&shared);
    let w1 = thread::spawn(move || worker(s1));

    let s2 = Arc::clone(&shared);
    let w2 = thread::spawn(move || worker(s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
