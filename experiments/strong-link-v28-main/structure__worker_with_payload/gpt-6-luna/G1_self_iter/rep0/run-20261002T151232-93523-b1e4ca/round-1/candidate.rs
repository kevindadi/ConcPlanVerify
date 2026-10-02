use std::sync::{Arc, Mutex};
use std::thread;

struct Counter {
    acc: usize,
}

fn compute() {
    let result = (0..1_000).fold(0usize, |sum, n| sum.wrapping_add(n));
    std::hint::black_box(result);
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(Counter { acc: 0 }));

    let h1 = {
        let m = Arc::clone(&m);
        thread::spawn(move || w1(m))
    };
    let h2 = {
        let m = Arc::clone(&m);
        thread::spawn(move || w2(m))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    assert_eq!(m.lock().unwrap().acc, 2);
    println!("DONE done=1");
}
