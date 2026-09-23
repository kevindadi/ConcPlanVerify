use std::sync::{Arc, Mutex};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
    c: Mutex<()>,
}

fn t1(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gb = locks.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(locks: Arc<Locks>) {
    let _gb = locks.b.lock().unwrap();
    let _gc = locks.c.lock().unwrap();
    drop(_gc);
    drop(_gb);
}

fn t3(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gc = locks.c.lock().unwrap();
    drop(_gc);
    drop(_ga);
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(()),
        b: Mutex::new(()),
        c: Mutex::new(()),
    });

    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let l3 = Arc::clone(&locks);

    let h1 = thread::spawn(move || t1(l1));
    let h2 = thread::spawn(move || t2(l2));
    let h3 = thread::spawn(move || t3(l3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
