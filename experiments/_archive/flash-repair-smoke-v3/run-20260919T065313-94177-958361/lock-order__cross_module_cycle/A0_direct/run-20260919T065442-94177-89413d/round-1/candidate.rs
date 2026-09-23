use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared state for the two resources, with a global lock ordering enforced
// by a single mutex + condvar pair. This avoids deadlock while still modeling
// cross-module resource acquisition.
struct Resources {
    a_held: bool,
    b_held: bool,
}

fn main() {
    let state = Arc::new((Mutex::new(Resources { a_held: false, b_held: false }), Condvar::new()));

    let s1 = Arc::clone(&state);
    let t1 = thread::spawn(move || {
        let (lock, cvar) = &*s1;
        let mut g = lock.lock().unwrap();
        // Task 1: acquire a then b.
        while g.a_held {
            g = cvar.wait(g).unwrap();
        }
        g.a_held = true;
        while g.b_held {
            g = cvar.wait(g).unwrap();
        }
        g.b_held = true;
        // Critical section: both resources held.
        g.a_held = false;
        g.b_held = false;
        cvar.notify_all();
    });

    let s2 = Arc::clone(&state);
    let t2 = thread::spawn(move || {
        let (lock, cvar) = &*s2;
        let mut g = lock.lock().unwrap();
        // Task 2: acquire b then a, but with consistent global ordering
        // enforced by the single mutex, so no deadlock occurs.
        while g.b_held {
            g = cvar.wait(g).unwrap();
        }
        g.b_held = true;
        while g.a_held {
            g = cvar.wait(g).unwrap();
        }
        g.a_held = true;
        // Critical section: both resources held.
        g.a_held = false;
        g.b_held = false;
        cvar.notify_all();
    });

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}
