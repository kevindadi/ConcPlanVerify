mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

const C_MAX: u8 = 2;

struct Counter {
    c: u8, // Invariant: 0 <= c <= C_MAX.
}

struct Shared {
    m: Mutex<Counter>,
}

fn w1(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    let next = guard.c.checked_add(1).unwrap();
    assert!(next <= C_MAX);
    guard.c = next;
}

fn w2(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    let next = guard.c.checked_add(1).unwrap();
    assert!(next <= C_MAX);
    guard.c = next;
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_observed("m#586", Counter { c: 0 }, __cir_obs_Counter),
    });

    let shared_for_w1 = Arc::clone(&shared);
    let shared_for_w2 = Arc::clone(&shared);

    let handle_w1 = cir_trace::spawn("w1#729", move || w1(shared_for_w1));
    let handle_w2 = cir_trace::spawn("w2#791", move || w2(shared_for_w2));

    handle_w1.join().unwrap();
    handle_w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_Counter(v: &Counter, r: &str) { cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
