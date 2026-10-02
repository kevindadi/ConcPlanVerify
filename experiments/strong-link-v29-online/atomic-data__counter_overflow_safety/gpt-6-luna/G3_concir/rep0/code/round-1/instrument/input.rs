use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let w1_m = Arc::clone(&m);
    let w1_handle = thread::spawn(move || w1(w1_m));

    let w2_m = Arc::clone(&m);
    let w2_handle = thread::spawn(move || w2(w2_m));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
}
