use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i64>>) {
    let mut c = m.lock().unwrap();
    let t = *c;
    *c = t + 1;
}

fn w2(m: Arc<Mutex<i64>>) {
    let mut c = m.lock().unwrap();
    let t = *c;
    *c = t + 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0i64));

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

    let done = 1;
    println!("DONE done={}", done);
}
