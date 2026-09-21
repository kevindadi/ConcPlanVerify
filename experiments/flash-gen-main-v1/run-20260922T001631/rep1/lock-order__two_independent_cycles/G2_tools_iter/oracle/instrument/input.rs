use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a1 = Arc::new(Mutex::new(()));
    let a2 = Arc::new(Mutex::new(()));
    let b1 = Arc::new(Mutex::new(()));
    let b2 = Arc::new(Mutex::new(()));

    let mut handles = Vec::new();

    {
        let l1 = Arc::clone(&a1);
        let l2 = Arc::clone(&a2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&a1);
        let l2 = Arc::clone(&a2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&b1);
        let l2 = Arc::clone(&b2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&b1);
        let l2 = Arc::clone(&b2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
