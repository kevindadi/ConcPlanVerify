use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let w1_handle = thread::spawn(move || {
        // main::w1
        let mut guard = m1.lock().unwrap();
        *guard += 1;
    });

    let m2 = Arc::clone(&m);
    let w2_handle = thread::spawn(move || {
        // main::w2
        let mut guard = m2.lock().unwrap();
        *guard += 1;
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
}
