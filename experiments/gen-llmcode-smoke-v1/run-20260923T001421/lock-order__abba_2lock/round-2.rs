use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let done1 = Arc::new(Mutex::new(false));
    let done2 = Arc::new(Mutex::new(false));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let done1_c = Arc::clone(&done1);

    let h1 = thread::spawn(move || {
        {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
            {
                let mut d = done1_c.lock().unwrap();
                *d = true;
            }
        }
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let done2_c = Arc::clone(&done2);

    let h2 = thread::spawn(move || {
        {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
            {
                let mut d = done2_c.lock().unwrap();
                *d = true;
            }
        }
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let v1 = *done1.lock().unwrap();
    let v2 = *done2.lock().unwrap();
    println!("DONE t1={} t2={}", v1 as u32, v2 as u32);
}
