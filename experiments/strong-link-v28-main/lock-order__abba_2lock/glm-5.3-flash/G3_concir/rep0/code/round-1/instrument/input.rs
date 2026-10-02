use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: &Mutex<i32>, b: &Mutex<i32>) {
    // lock main::a
    let _ga = a.lock().unwrap();
    // lock main::b
    let _gb = b.lock().unwrap();
    // critical work
    let work = 1;
    let _ = work;
    // unlock main::b
    drop(_gb);
    // unlock main::a
    drop(_ga);
}

fn t2(a: &Mutex<i32>, b: &Mutex<i32>) {
    // lock main::a
    let _ga = a.lock().unwrap();
    // lock main::b
    let _gb = b.lock().unwrap();
    // critical work
    let work = 1;
    let _ = work;
    // unlock main::b
    drop(_gb);
    // unlock main::a
    drop(_ga);
}

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(&a2, &b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
