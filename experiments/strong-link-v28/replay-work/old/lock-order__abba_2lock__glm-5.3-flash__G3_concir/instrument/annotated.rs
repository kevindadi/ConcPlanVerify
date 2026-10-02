mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(locks: Arc<Locks>) {
    // mutex_lock main::a
    let _ga = locks.a.lock().unwrap();
    // mutex_lock main::b
    let _gb = locks.b.lock().unwrap();
    // critical work while holding both locks
    // mutex_unlock main::b
    drop(_gb);
    // mutex_unlock main::a
    drop(_ga);
}

fn t2(locks: Arc<Locks>) {
    // mutex_lock main::a
    let _ga = locks.a.lock().unwrap();
    // mutex_lock main::b
    let _gb = locks.b.lock().unwrap();
    // critical work while holding both locks
    // mutex_unlock main::b
    drop(_gb);
    // mutex_unlock main::a
    drop(_ga);
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("a#748", ()),
        b: Mutex::new_named("b#775", ()),
    });

    // spawn main::t1 (handle h1)
    let h1 = {
        let locks = Arc::clone(&locks);
        cir_trace::spawn("t1#890", move || t1(locks))
    };

    // spawn main::t2 (handle h2)
    let h2 = {
        let locks = Arc::clone(&locks);
        cir_trace::spawn("t2#1028", move || t2(locks))
    };

    // join h1
    h1.join().unwrap();
    // join h2
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
