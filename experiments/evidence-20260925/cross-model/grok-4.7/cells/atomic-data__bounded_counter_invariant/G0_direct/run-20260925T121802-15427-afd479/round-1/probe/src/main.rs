use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = thread::spawn(move || w1(m1));
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
