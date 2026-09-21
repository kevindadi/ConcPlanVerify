use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));
    let lock3 = Arc::new(Mutex::new(()));

    // To avoid deadlock (R5, R8, R9), impose a global ordering on the locks
    // and always acquire them in that order. Worker 3 needs lock3 and lock1,
    // so it acquires lock1 first, then lock3.
    let l1 = Arc::clone(&lock1);
    let l2 = Arc::clone(&lock2);
    let w1 = thread::spawn(move || {
        let _a = l1.lock().unwrap();
        let _b = l2.lock().unwrap();
        // critical work
    });

    let l2 = Arc::clone(&lock2);
    let l3 = Arc::clone(&lock3);
    let w2 = thread::spawn(move || {
        let _a = l2.lock().unwrap();
        let _b = l3.lock().unwrap();
        // critical work
    });

    let l1 = Arc::clone(&lock1);
    let l3 = Arc::clone(&lock3);
    let w3 = thread::spawn(move || {
        let _a = l1.lock().unwrap();
        let _b = l3.lock().unwrap();
        // critical work
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
