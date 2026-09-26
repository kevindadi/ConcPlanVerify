use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let gb = b.lock().unwrap();
    let gc = c.lock().unwrap();
    drop(gc);
    drop(gb);
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let ga = a.lock().unwrap();
    let gc = c.lock().unwrap();
    drop(gc);
    drop(ga);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(&a1, &b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = thread::spawn(move || t2(&b2, &c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = thread::spawn(move || t3(&a3, &c3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
