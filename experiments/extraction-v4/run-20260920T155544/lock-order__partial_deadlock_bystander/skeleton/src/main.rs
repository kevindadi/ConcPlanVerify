//! Reference Rust for tests/e2e/partial_deadlock/fixed.json.
//! Fix: the semaphore cross-handshake is removed and both workers take the
//! mutexes in the same global order (a then b), so both reach return.

mod cir_trace;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let (ma, mb) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let worker_a = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        cir_trace::ev(&cir_trace::tag_str(), "L2"); let ga = ma.lock().unwrap(); // s1: lock mtx_a
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let gb = mb.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3
        drop(ga); // s4
    });

    let (ma2, mb2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    cir_trace::ev(&cir_trace::tag_str(), "L4"); let worker_b = thread::spawn(move || {cir_trace::set_tag("tL4"); 
        cir_trace::ev(&cir_trace::tag_str(), "L5"); let ga = ma2.lock().unwrap(); // s1: lock mtx_a (same order)
        cir_trace::ev(&cir_trace::tag_str(), "L6"); let gb = mb2.lock().unwrap(); // s2: lock mtx_b
        drop(gb); // s3
        drop(ga); // s4
    });

    // Detached bystander keeps looping; main never joins it.
    cir_trace::ev(&cir_trace::tag_str(), "L7"); thread::spawn(move || { cir_trace::set_tag("tL7"); loop {
        thread::sleep(Duration::from_millis(10)); // s1/s2: nop loop
    }});

    // Goals: worker_a.ret and worker_b.ret are both reachable.
    cir_trace::ev(&cir_trace::tag_str(), "L8"); worker_a.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L9"); worker_b.join().unwrap();
cir_trace::finish(); }
