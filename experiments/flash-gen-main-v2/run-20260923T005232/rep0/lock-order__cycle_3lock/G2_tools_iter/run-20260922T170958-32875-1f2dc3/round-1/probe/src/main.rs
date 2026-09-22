use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // Global ordering to prevent circular wait (deadlock):
    // a < b < c. Each worker acquires its locks in this order.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // critical work
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let t2 = thread::spawn(move || {
        let _gb = b2.lock().unwrap();
        let _gc = c2.lock().unwrap();
        // critical work
    });

    let c3 = Arc::clone(&c);
    let a3 = Arc::clone(&a);
    let t3 = thread::spawn(move || {
        // Needs c and a; acquire in global order a < c.
        let _ga = a3.lock().unwrap();
        let _gc = c3.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
