use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let done = Arc::new(Semaphore::new(0));

    let m1 = Arc::clone(&m);
    let d1 = Arc::clone(&done);
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        *guard += 1;
        drop(guard);
        d1.acquire();
    });

    let m2 = Arc::clone(&m);
    let d2 = Arc::clone(&done);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        *guard += 1;
        drop(guard);
        d2.acquire();
    });

    // Supervising task: wait for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    let value = {
        let guard = m.lock().unwrap();
        *guard
    };

    println!("DONE done={}", value);
}
