use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    acc: i32,
}

fn compute() {
    let t = 1 + 1;
    let _u = t * 2;
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
}

fn print() {}

fn main() {
    let m = Arc::new(Mutex::new(Shared { acc: 0 }));
    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));
    h1.join().unwrap();
    h2.join().unwrap();
    print();
    println!("DONE done=1");
}
