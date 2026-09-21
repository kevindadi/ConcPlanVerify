use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));

    let l1a = Arc::clone(&lock1);
    let l2a = Arc::clone(&lock2);
    let l1b = Arc::clone(&lock1);
    let l2b = Arc::clone(&lock2);

    // Both workers acquire locks in the same global order to prevent deadlock.
    let t1 = thread::spawn(move || {
        let _g1 = l1a.lock().unwrap();
        let _g2 = l2a.lock().unwrap();
        // critical work
    });

    let t2 = thread::spawn(move || {
        let _g1 = l1b.lock().unwrap();
        let _g2 = l2b.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
