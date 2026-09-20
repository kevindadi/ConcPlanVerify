mod cir_trace;
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
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let t1 = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        let (lock, cvar) = &*s1;
        cir_trace::ev(&cir_trace::tag_str(), "L2"); let mut guard = lock.lock().unwrap();
        // Task 1: acquire a then b.
        while guard.a_held {
            cir_trace::ev(&cir_trace::tag_str(), "L3"); guard = cvar.wait(guard).unwrap();
        }
        guard.a_held = true;
        while guard.b_held {
            cir_trace::ev(&cir_trace::tag_str(), "L4"); guard = cvar.wait(guard).unwrap();
        }
        guard.b_held = true;
        // Both held; release.
        guard.a_held = false;
        guard.b_held = false;
        cir_trace::ev(&cir_trace::tag_str(), "L5"); cvar.notify_all();
    });

    let s2 = Arc::clone(&state);
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let t2 = thread::spawn(move || {cir_trace::set_tag("tL6"); 
        let (lock, cvar) = &*s2;
        cir_trace::ev(&cir_trace::tag_str(), "L7"); let mut guard = lock.lock().unwrap();
        // Task 2: acquire b then a.
        while guard.b_held {
            cir_trace::ev(&cir_trace::tag_str(), "L8"); guard = cvar.wait(guard).unwrap();
        }
        guard.b_held = true;
        while guard.a_held {
            cir_trace::ev(&cir_trace::tag_str(), "L9"); guard = cvar.wait(guard).unwrap();
        }
        guard.a_held = true;
        // Both held; release.
        guard.a_held = false;
        guard.b_held = false;
        cir_trace::ev(&cir_trace::tag_str(), "L10"); cvar.notify_all();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L11"); t1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L12"); t2.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
