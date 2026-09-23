use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let worker1 = thread::spawn(move || {
        let mut ga = a1.lock().unwrap();
        let mut gb = b1.lock().unwrap();
        *ga += 1;
        *gb += 1;
        drop(gb);
        drop(ga);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let worker2 = thread::spawn(move || {
        let mut ga = a2.lock().unwrap();
        let mut gb = b2.lock().unwrap();
        *ga += 1;
        *gb += 1;
        drop(gb);
        drop(ga);
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("A = {}, B = {}", *a.lock().unwrap(), *b.lock().unwrap());
}
