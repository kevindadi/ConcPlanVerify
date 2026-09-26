use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0u32));
    let c = Arc::new(Mutex::new(0u32));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let w1 = thread::spawn(move || {
        let _guard = m1.lock().unwrap();
        let mut val = c1.lock().unwrap();
        if *val < 1 {
            *val += 1;
        }
    });

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let w2 = thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        let mut val = c2.lock().unwrap();
        if *val < 1 {
            *val += 1;
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
}
