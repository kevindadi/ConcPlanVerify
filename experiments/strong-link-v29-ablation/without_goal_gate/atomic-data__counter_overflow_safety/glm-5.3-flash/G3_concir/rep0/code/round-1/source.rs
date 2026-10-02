use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    if *guard < 1 {
        *guard = *guard + 1;
    }
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    if *guard < 1 {
        *guard = *guard + 1;
    }
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(&m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(&m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let c = m.lock().unwrap();
    println!("DONE done={}", *c);
}
