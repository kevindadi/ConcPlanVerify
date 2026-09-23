mod concir_sync;

use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    acc: i32,
}

fn compute() {
    let x: i32 = 1;
    let _y: i32 = x + 1;
}

fn w1(m: Arc<Mutex<Shared>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let tmp = guard.acc;
        let tmp2 = tmp + 1;
        guard.acc = tmp2;
    }
}

fn w2(m: Arc<Mutex<Shared>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let tmp = guard.acc;
        let tmp2 = tmp + 1;
        guard.acc = tmp2;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { acc: 0 }));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = {
        let guard = m.lock().unwrap();
        guard.acc
    };

    println!("DONE done={}", done);
}
