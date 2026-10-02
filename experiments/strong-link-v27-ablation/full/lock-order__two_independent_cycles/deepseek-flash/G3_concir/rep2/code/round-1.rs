use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || {
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        let x: i32 = 1;
        let _ = x;
        drop(gb);
        drop(ga);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || {
        let ga = a2.lock().unwrap();
        let gb = b2.lock().unwrap();
        let y: i32 = 2;
        let _ = y;
        drop(gb);
        drop(ga);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let t3 = thread::spawn(move || {
        let gc = c3.lock().unwrap();
        let gd = d3.lock().unwrap();
        let z: i32 = 3;
        let _ = z;
        drop(gd);
        drop(gc);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let t4 = thread::spawn(move || {
        let gc = c4.lock().unwrap();
        let gd = d4.lock().unwrap();
        let w: i32 = 4;
        let _ = w;
        drop(gd);
        drop(gc);
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
}
