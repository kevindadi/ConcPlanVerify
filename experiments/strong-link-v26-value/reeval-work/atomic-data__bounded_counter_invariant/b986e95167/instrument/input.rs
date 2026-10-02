use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0u8));

    let w1 = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        })
    };

    let w2 = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            let mut guard = m.lock().unwrap();
            *guard += 1;
        })
    };

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
