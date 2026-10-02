use std::sync::{Arc, Mutex};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    let current = *guard;
    *guard = current + 1;
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    let current = *guard;
    *guard = current + 1;
}

fn main() {
    let c = Arc::new(Mutex::new(0));

    let w1_handle = {
        let c = Arc::clone(&c);
        thread::spawn(move || w1(c))
    };
    let w2_handle = {
        let c = Arc::clone(&c);
        thread::spawn(move || w2(c))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
}
