use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Four locks shared among the workers.
    let l0 = Arc::new(Mutex::new(()));
    let l1 = Arc::new(Mutex::new(()));
    let l2 = Arc::new(Mutex::new(()));
    let l3 = Arc::new(Mutex::new(()));

    let mut handles = Vec::new();

    // First pair: workers 0 and 1 both need l0 and l1, in the same order.
    {
        let a = Arc::clone(&l0);
        let b = Arc::clone(&l1);
        handles.push(thread::spawn(move || {
            let _g0 = a.lock().unwrap();
            let _g1 = b.lock().unwrap();
            // work while holding both locks
        }));
    }
    {
        let a = Arc::clone(&l0);
        let b = Arc::clone(&l1);
        handles.push(thread::spawn(move || {
            let _g0 = a.lock().unwrap();
            let _g1 = b.lock().unwrap();
            // work while holding both locks
        }));
    }

    // Second pair: workers 2 and 3 both need l2 and l3, in the same order.
    {
        let a = Arc::clone(&l2);
        let b = Arc::clone(&l3);
        handles.push(thread::spawn(move || {
            let _g2 = a.lock().unwrap();
            let _g3 = b.lock().unwrap();
            // work while holding both locks
        }));
    }
    {
        let a = Arc::clone(&l2);
        let b = Arc::clone(&l3);
        handles.push(thread::spawn(move || {
            let _g2 = a.lock().unwrap();
            let _g3 = b.lock().unwrap();
            // work while holding both locks
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
