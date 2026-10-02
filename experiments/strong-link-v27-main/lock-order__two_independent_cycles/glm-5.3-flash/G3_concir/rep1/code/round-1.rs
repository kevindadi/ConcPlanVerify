use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

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

    let c1 = Arc::clone(&c);
    let d1 = Arc::clone(&d);
    let h3 = thread::spawn(move || {
        let _gc = c1.lock().unwrap();
        let _gd = d1.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    let c2 = Arc::clone(&c);
    let d2 = Arc::clone(&d);
    let h4 = thread::spawn(move || {
        let _gc = c2.lock().unwrap();
        let _gd = d2.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
