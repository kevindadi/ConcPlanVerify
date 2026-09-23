use std::sync::{Arc, Mutex};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
    c: Mutex<()>,
    d: Mutex<()>,
}

fn t1(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gb = locks.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gb = locks.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t3(locks: Arc<Locks>) {
    let _gc = locks.c.lock().unwrap();
    let _gd = locks.d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn t4(locks: Arc<Locks>) {
    let _gc = locks.c.lock().unwrap();
    let _gd = locks.d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(()),
        b: Mutex::new(()),
        c: Mutex::new(()),
        d: Mutex::new(()),
    });

    let l1 = Arc::clone(&locks);
    let h1 = thread::spawn(move || t1(l1));

    let l2 = Arc::clone(&locks);
    let h2 = thread::spawn(move || t2(l2));

    let l3 = Arc::clone(&locks);
    let h3 = thread::spawn(move || t3(l3));

    let l4 = Arc::clone(&locks);
    let h4 = thread::spawn(move || t4(l4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
