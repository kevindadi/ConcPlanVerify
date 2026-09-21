use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));
    let lock3 = Arc::new(Mutex::new(()));

    // To avoid deadlock (R5, R8, R9), impose a global ordering on lock acquisition.
    // Order: lock1 < lock2 < lock3.
    // Worker 1 needs lock1 and lock2 -> acquire lock1 then lock2.
    // Worker 2 needs lock2 and lock3 -> acquire lock2 then lock3.
    // Worker 3 needs lock3 and lock1 -> acquire lock1 then lock3 (ordered).

    let l1a = Arc::clone(&lock1);
    let l2a = Arc::clone(&lock2);
    let h1 = thread::spawn(move || {
        let _g1 = l1a.lock().unwrap();
        let _g2 = l2a.lock().unwrap();
        // critical work
    });

    let l2b = Arc::clone(&lock2);
    let l3b = Arc::clone(&lock3);
    let h2 = thread::spawn(move || {
        let _g2 = l2b.lock().unwrap();
        let _g3 = l3b.lock().unwrap();
        // critical work
    });

    let l1c = Arc::clone(&lock1);
    let l3c = Arc::clone(&lock3);
    let h3 = thread::spawn(move || {
        let _g1 = l1c.lock().unwrap();
        let _g3 = l3c.lock().unwrap();
        // critical work
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
