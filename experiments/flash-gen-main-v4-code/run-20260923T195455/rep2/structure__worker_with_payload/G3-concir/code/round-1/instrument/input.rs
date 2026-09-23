use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x: i64 = 1;
    let _y: i64 = x + 1;
}

fn w1(m: Arc<Mutex<i64>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp: i64 = *guard;
        tmp = tmp + 1;
        *guard = tmp;
    }
}

fn w2(m: Arc<Mutex<i64>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp: i64 = *guard;
        tmp = tmp + 1;
        *guard = tmp;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0i64));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
