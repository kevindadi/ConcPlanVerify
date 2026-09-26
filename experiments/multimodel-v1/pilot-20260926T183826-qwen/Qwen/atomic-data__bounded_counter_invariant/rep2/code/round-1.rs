use std::sync::{Arc, Mutex};

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let w1 = std::thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        *guard += 1;
    });

    let m2 = Arc::clone(&m);
    let w2 = std::thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        *guard += 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = if *m.lock().unwrap() == 2 { 1 } else { 0 };
    println!("DONE done={}", done);
}
