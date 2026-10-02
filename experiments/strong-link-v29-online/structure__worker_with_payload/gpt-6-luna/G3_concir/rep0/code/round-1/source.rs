use std::sync::{Arc, Mutex};
use std::thread;

struct State {
    acc: i32,
}

fn compute() {
    let mut tmp = 0;
    tmp = tmp + 1;
}

fn w1(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(State { acc: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = i32::from(m.lock().unwrap().acc == 2);
    println!("DONE done={}", done);
}
