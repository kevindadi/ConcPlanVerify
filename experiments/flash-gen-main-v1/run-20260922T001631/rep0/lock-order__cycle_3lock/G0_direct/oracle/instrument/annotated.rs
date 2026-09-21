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

    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let l3 = Arc::clone(&locks);

    // Worker 1 needs locks a and b.
    let w1 = cir_trace::spawn("w1", move || {
        let _ga = l1.a.lock().unwrap();
        let _gb = l1.b.lock().unwrap();
        // critical work
    });

    // Worker 2 needs locks b and c.
    let w2 = cir_trace::spawn("w2", move || {
        let _gb = l2.b.lock().unwrap();
        let _gc = l2.c.lock().unwrap();
        // critical work
    });

    // Worker 3 needs locks c and a.
    let w3 = cir_trace::spawn("w3", move || {
        let _gc = l3.c.lock().unwrap();
        let _ga = l3.a.lock().unwrap();
        // critical work
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
