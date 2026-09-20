use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Shared state for the two resources, with a global lock ordering enforced
// by a single mutex + condvar pair. This avoids deadlock while still
// modelling cross-module resource acquisition.
struct Resources {
    a_held: bool,
    b_held: bool,
}

fn main() {
    let state = Arc::new((Mutex::new(Resources { a_held: false, b_held: false }), Condvar::new()));

    let s1 = Arc::clone(&state);
    let t1 = thread::spawn(move || {
        let (lock, cvar) = &*s1;
        let mut guard = lock.lock().unwrap();
        // Task 1: acquire a then b.
        while guard.a_held {
            guard = cvar.wait(guard).unwrap();
        }
        guard.a_held = true;
        while guard.b_held {
            guard = cvar.wait(guard).unwrap();
        }
        guard.b_held = true;
        // Both held; release.
        guard.a_held = false;
        guard.b_held = false;
        cvar.notify_all();
    });

    let s2 = Arc::clone(&state);
    let t2 = thread::spawn(move || {
        let (lock, cvar) = &*s2;
        let mut guard = lock.lock().unwrap();
        // Task 2: acquire b then a.
        while guard.b_held {
            guard = cvar.wait(guard).unwrap();
        }
        guard.b_held = true;
        while guard.a_held {
            guard = cvar.wait(guard).unwrap();
        }
        guard.a_held = true;
        // Both held; release.
        guard.a_held = false;
        guard.b_held = false;
        cvar.notify_all();
    });

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}
