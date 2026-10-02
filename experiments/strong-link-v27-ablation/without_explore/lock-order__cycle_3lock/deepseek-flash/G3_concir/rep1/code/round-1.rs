use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // t1 needs a then b
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let h1 = thread::spawn(move || {
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        let _x: i32 = 1;
        drop(gb);
        drop(ga);
    });

    // t2 needs b then c
    let (b2, c2) = (Arc::clone(&b), Arc::clone(&c));
    let h2 = thread::spawn(move || {
        let gb = b2.lock().unwrap();
        let gc = c2.lock().unwrap();
        let _x: i32 = 1;
        drop(gc);
        drop(gb);
    });

    // t3 needs a then c
    let (a3, c3) = (Arc::clone(&a), Arc::clone(&c));
    let h3 = thread::spawn(move || {
        let ga = a3.lock().unwrap();
        let gc = c3.lock().unwrap();
        let _x: i32 = 1;
        drop(gc);
        drop(ga);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
