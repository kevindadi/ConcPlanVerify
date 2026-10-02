//! Reference Rust for tests/e2e/mutex_deadlock/fixed.json.
//! Fix: both workers acquire the mutexes in the same global order (a then b).

mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let mtx_a = Arc::new(Mutex::new_named("mtx_a_mutex0#231", ()));
    let mtx_b = Arc::new(Mutex::new_named("mtx_b_mutex0#273", ()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = cir_trace::spawn("w1#358", move || {
        let ga = a1.lock().unwrap(); // s1: lock mtx_a
        let gb = b1.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3: drop mtx_b
        drop(ga); // s4: drop mtx_a
    });

    let (a2, b2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w2 = cir_trace::spawn("w2#647", move || {
        let ga = a2.lock().unwrap(); // s1: lock mtx_a (same order as w1)
        let gb = b2.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3: drop mtx_b
        drop(ga); // s4: drop mtx_a
    });

    w1.join().unwrap();
    w2.join().unwrap();
 cir_trace::finish();}
