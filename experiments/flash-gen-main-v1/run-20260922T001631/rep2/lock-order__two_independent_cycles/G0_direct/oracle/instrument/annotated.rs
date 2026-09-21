mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Locks {
    a1: Mutex<()>,
    a2: Mutex<()>,
    b1: Mutex<()>,
    b2: Mutex<()>,
}

fn worker_pair1(id: usize, locks: Arc<Locks>, done: Arc<Mutex<usize>>) {
    // Both workers in pair 1 acquire a1 then a2 (same relative sequence).
    let _g1 = locks.a1.lock().unwrap();
    let _g2 = locks.a2.lock().unwrap();
    // Hold both locks while working.
    // Release both before finishing (drop order).
    drop(_g2);
    drop(_g1);
    let mut d = done.lock().unwrap();
    *d += 1;
    let _ = id;
}

fn worker_pair2(id: usize, locks: Arc<Locks>, done: Arc<Mutex<usize>>) {
    // Both workers in pair 2 acquire b1 then b2 (same relative sequence).
    let _g1 = locks.b1.lock().unwrap();
    let _g2 = locks.b2.lock().unwrap();
    drop(_g2);
    drop(_g1);
    let mut d = done.lock().unwrap();
    *d += 1;
    let _ = id;
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a1: Mutex::new_named("locks_mutex0", ()),
        a2: Mutex::new_named("locks_mutex1", ()),
        b1: Mutex::new_named("locks_mutex2", ()),
        b2: Mutex::new_named("locks_mutex3", ()),
    });
    let done = Arc::new(Mutex::new_named("done_mutex0", 0usize));

    let mut handles = Vec::new();

    for id in 0..2 {
        let l = Arc::clone(&locks);
        let d = Arc::clone(&done);
        handles.push(thread::spawn(move || worker_pair1(id, l, d)));
    }
    for id in 0..2 {
        let l = Arc::clone(&locks);
        let d = Arc::clone(&done);
        handles.push(thread::spawn(move || worker_pair2(id, l, d)));
    }

    for h in handles {
        h.join().unwrap();
    }

    let d = done.lock().unwrap();
    if *d == 4 {
        println!("DONE done=1");
    }
 cir_trace::finish();}
