use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
    drop(c);
}

fn main() {
    // Shared variable c (range 0..=2, init 0) lives inside mutex m.
    let m = Arc::new(Mutex::new(0i32));

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

    println!("DONE done=1");
}
