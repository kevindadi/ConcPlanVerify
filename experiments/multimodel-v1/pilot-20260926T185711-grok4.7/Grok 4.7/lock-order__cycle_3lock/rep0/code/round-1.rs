use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    let _ = work;
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let mut work = 0;
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    work = 1;
    drop(guard_c);
    drop(guard_b);
    let _ = work;
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    work = 1;
    drop(guard_c);
    drop(guard_a);
    let _ = work;
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || {
        t1(&a1, &b1);
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = thread::spawn(move || {
        t2(&b2, &c2);
    });

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = thread::spawn(move || {
        t3(&a3, &c3);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
