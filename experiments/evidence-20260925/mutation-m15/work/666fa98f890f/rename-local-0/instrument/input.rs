use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a_kept = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a_kept);
    let b1 = Arc::clone(&b);
    let w1 = thread::spawn(move || {
        {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
        }
    });

    let a2 = Arc::clone(&a_kept);
    let b2 = Arc::clone(&b);
    let w2 = thread::spawn(move || {
        {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
