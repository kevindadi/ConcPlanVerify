use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Three locks shared among the workers.
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));
    let lock3 = Arc::new(Mutex::new(()));

    // To avoid deadlock (R5, R8, R9), impose a global lock ordering:
    // every worker acquires its locks in the order lock1 < lock2 < lock3.
    // Worker 1 needs lock1 and lock2 -> acquire lock1 then lock2.
    // Worker 2 needs lock2 and lock3 -> acquire lock2 then lock3.
    // Worker 3 needs lock3 and lock1 -> acquire lock1 then lock3 (ordered).
    let mut handles = Vec::new();

    {
        let l1 = Arc::clone(&lock1);
        let l2 = Arc::clone(&lock2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
            // critical work with both locks held
        }));
    }

    {
        let l2 = Arc::clone(&lock2);
        let l3 = Arc::clone(&lock3);
        handles.push(thread::spawn(move || {
            let _g2 = l2.lock().unwrap();
            let _g3 = l3.lock().unwrap();
            // critical work with both locks held
        }));
    }

    {
        let l1 = Arc::clone(&lock1);
        let l3 = Arc::clone(&lock3);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g3 = l3.lock().unwrap();
            // critical work with both locks held
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
