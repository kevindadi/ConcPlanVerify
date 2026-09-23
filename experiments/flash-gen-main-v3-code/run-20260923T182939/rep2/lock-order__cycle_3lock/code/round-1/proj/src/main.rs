mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", ()),
        b: Mutex::new_named("locks_mutex1", ()),
        c: Mutex::new_named("locks_mutex2", ()),
    });

    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let l3 = Arc::clone(&locks);

    let h1 = cir_trace::spawn("t1", move || t1(l1));
    let h2 = cir_trace::spawn("t2", move || t2(l2));
    let h3 = cir_trace::spawn("t3", move || t3(l3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
