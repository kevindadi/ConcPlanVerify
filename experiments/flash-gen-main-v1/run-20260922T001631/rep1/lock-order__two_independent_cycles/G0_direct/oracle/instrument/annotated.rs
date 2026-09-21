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

fn worker_pair(
    l1: Arc<Mutex<()>>,
    l2: Arc<Mutex<()>>,
    name: &'static str,
) {
    // Acquire in a fixed relative order to avoid deadlock within the pair.
    let g1 = l1.lock().unwrap();
    let g2 = l2.lock().unwrap();
    // Hold both locks simultaneously while working.
    let _ = (name, &*g1, &*g2);
    drop(g2);
    drop(g1);
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a1: Mutex::new_named("locks_mutex0", ()),
        a2: Mutex::new_named("locks_mutex1", ()),
        b1: Mutex::new_named("locks_mutex2", ()),
        b2: Mutex::new_named("locks_mutex3", ()),
    });

    let mut handles = Vec::new();

    // First pair: workers 1 and 2, both take a1 then a2.
    for i in 0..2 {
        let l1 = Arc::clone(&locks.a1);
        let l2 = Arc::clone(&locks.a2);
        let name = if i == 0 { "w1" } else { "w2" };
        handles.push(thread::spawn(move || {
            worker_pair(l1, l2, name);
        }));
    }

    // Second pair: workers 3 and 4, both take b1 then b2.
    for i in 0..2 {
        let l1 = Arc::clone(&locks.b1);
        let l2 = Arc::clone(&locks.b2);
        let name = if i == 0 { "w3" } else { "w4" };
        handles.push(thread::spawn(move || {
            worker_pair(l1, l2, name);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
