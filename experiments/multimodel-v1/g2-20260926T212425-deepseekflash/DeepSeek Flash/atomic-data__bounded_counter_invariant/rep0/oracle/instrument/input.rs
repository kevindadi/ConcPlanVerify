use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let done = Arc::new(Mutex::new(0i32));
    let sem = Arc::new(Semaphore::new(2));

    let m1 = Arc::clone(&m);
    let s1 = Arc::clone(&sem);
    let w1 = thread::spawn(move || {
        let permit = s1.acquire();
        {
            let mut guard = m1.lock().unwrap();
            *guard += 1;
        }
        permit.release();
    });

    let m2 = Arc::clone(&m);
    let s2 = Arc::clone(&sem);
    let w2 = thread::spawn(move || {
        let permit = s2.acquire();
        {
            let mut guard = m2.lock().unwrap();
            *guard += 1;
        }
        permit.release();
    });

    // Supervising task: wait for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    let value = {
        let mut guard = done.lock().unwrap();
        *guard = 1;
        *guard
    };

    println!("DONE done={}", value);
}
