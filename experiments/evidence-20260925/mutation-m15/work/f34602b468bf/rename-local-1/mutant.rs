use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b_kept = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b_kept);
    let t1 = thread::spawn(move || {
        {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
        }
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b_kept);
    let t2 = thread::spawn(move || {
        {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
