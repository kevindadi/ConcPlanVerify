use std::sync::{Arc, Mutex};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    *guard = *guard + 1;
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    *guard = *guard + 1;
}

fn main() {
    let c = Arc::new(Mutex::new(0));

    let c1 = Arc::clone(&c);
    let handle1 = thread::spawn(move || w1(c1));

    let c2 = Arc::clone(&c);
    let handle2 = thread::spawn(move || w2(c2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
}
