use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));
    let done = Arc::new(Mutex::new(0i32));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = thread::spawn(move || {
        let _gc = c3.lock().unwrap();
        let _gd = d3.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = thread::spawn(move || {
        let _gc = c4.lock().unwrap();
        let _gd = d4.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    {
        let mut g = done.lock().unwrap();
        *g = 1;
    }

    let val = *done.lock().unwrap();
    println!("DONE done={}", val);
}
