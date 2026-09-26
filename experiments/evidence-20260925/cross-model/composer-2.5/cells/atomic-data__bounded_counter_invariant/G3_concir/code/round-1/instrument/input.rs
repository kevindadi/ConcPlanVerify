use std::sync::{Arc, Mutex};
use std::thread;

fn println() {
    println!("DONE done=1");
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let v = *guard;
    *guard = v + 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let v = *guard;
    *guard = v + 1;
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(m1));
    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || w2(m2));
    h1.join().unwrap();
    h2.join().unwrap();
    println();
}
