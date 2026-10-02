mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: &Arc<Mutex<Shared>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // read_shared c
    let c = guard.c;
    // write_shared c = c + 1
    guard.c = c + 1;
    // mutex_unlock m (guard drops at end of scope)
    drop(guard);
}

fn w2(m: &Arc<Mutex<Shared>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // read_shared c
    let c = guard.c;
    // write_shared c = c + 1
    guard.c = c + 1;
    // mutex_unlock m (guard drops at end of scope)
    drop(guard);
}

fn main() { crate::cir_trace::init();
    // shared resource c (range 0..=2, starts at 0) protected by lock m
    let m = Arc::new(Mutex::new_observed("m_mutex0#702", Shared { c: 0 }, __cir_obs_Shared));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#768", move || w1(&m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#843", move || w2(&m2));

    // supervising task waits for both workers to finish
    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    let done = { m.lock().unwrap().c / 2 };
    println!("DONE done={}", done);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::c", r), v.c as i64); }
