use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    let t: i32 = *c;
    *c = t + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    let t: i32 = *c;
    *c = t + 1;
}

fn main() {
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

    let h1 = thread::spawn({
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let h2 = thread::spawn({
        let m = Arc::clone(&m);
        move || w2(m)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done: i32 = 1;
    println!("DONE done={}", done);
}
