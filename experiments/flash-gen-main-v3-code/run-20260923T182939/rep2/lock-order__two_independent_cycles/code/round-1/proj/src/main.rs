mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", ()),
        b: Mutex::new_named("locks_mutex1", ()),
        c: Mutex::new_named("locks_mutex2", ()),
        d: Mutex::new_named("locks_mutex3", ()),
    });

    let l1 = Arc::clone(&locks);
    let h1 = cir_trace::spawn("t1", move || t1(l1));

    let l2 = Arc::clone(&locks);
    let h2 = cir_trace::spawn("t2", move || t2(l2));

    let l3 = Arc::clone(&locks);
    let h3 = cir_trace::spawn("t3", move || t3(l3));

    let l4 = Arc::clone(&locks);
    let h4 = cir_trace::spawn("t4", move || t4(l4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
