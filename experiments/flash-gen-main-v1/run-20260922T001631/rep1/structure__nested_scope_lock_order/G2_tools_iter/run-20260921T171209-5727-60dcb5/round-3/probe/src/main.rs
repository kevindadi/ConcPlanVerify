use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let outer = thread::spawn(move || {
        let inner1 = thread::spawn(move || {
            let mut ga = a1.lock().unwrap();
            let mut gb = b1.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        let inner2 = thread::spawn(move || {
            let mut ga = a2.lock().unwrap();
            let mut gb = b2.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        inner1.join().unwrap();
        inner2.join().unwrap();
    });

    outer.join().unwrap();

    let done = *a.lock().unwrap();
    println!("DONE done={}", done);
}
