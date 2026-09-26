use std::sync::{Arc, Mutex};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0_i32));

    let worker1 = {
        let c = Arc::clone(&m);
        thread::spawn(move || w1(c))
    };
    let worker2 = {
        let c = Arc::clone(&m);
        thread::spawn(move || w2(c))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done={}", *m.lock().unwrap());
}
