mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
    c: Mutex<()>,
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", ()),
        b: Mutex::new_named("locks_mutex1", ()),
        c: Mutex::new_named("locks_mutex2", ()),
    });

    // To avoid deadlock (R5), impose a global ordering on lock acquisition.
    // Worker 1 needs a and b: acquire a then b.
    // Worker 2 needs b and c: acquire b then c.
    // Worker 3 needs c and a: acquire a then c (order a before c).
    let l1 = Arc::clone(&locks);
    let t1 = cir_trace::spawn("t1", move || {
        let _ga = l1.a.lock().unwrap();
        let _gb = l1.b.lock().unwrap();
        // critical work
    });

    let l2 = Arc::clone(&locks);
    let t2 = cir_trace::spawn("t2", move || {
        let _gb = l2.b.lock().unwrap();
        let _gc = l2.c.lock().unwrap();
        // critical work
    });

    let l3 = Arc::clone(&locks);
    let t3 = cir_trace::spawn("t3", move || {
        let _ga = l3.a.lock().unwrap();
        let _gc = l3.c.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
