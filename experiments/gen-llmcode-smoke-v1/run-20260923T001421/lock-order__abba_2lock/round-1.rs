use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let done1 = Arc::new(Mutex::new(false));
    let done2 = Arc::new(Mutex::new(false));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let done1_clone = Arc::clone(&done1);
    let h1 = thread::spawn(move || {
        {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
            *done1_clone.lock().unwrap() = true;
        }
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let done2_clone = Arc::clone(&done2);
    let h2 = thread::spawn(move || {
        {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
            *done2_clone.lock().unwrap() = true;
        }
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let d1 = *done1.lock().unwrap();
    let d2 = *done2.lock().unwrap();
    println!("DONE t1={} t2={}", d1 as u32, d2 as u32);
}
