use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c = *c + 1;
    }
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c = *c + 1;
    }
    drop(c);
}

fn main() {
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

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
