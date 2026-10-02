mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resource `acc` (init 0) lives inside the mutex `m` that guards it.
fn compute(tmp: i32) -> i32 {
    let tmp = tmp + 1;
    let tmp = tmp * 2;
    tmp
}

fn w1(m: &Arc<Mutex<i32>>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // call main::compute (sequential, local only)
    let _result = compute(*guard);
    // write_shared main::acc = 1 (still holding the mutex)
    *guard = 1;
    // mutex_unlock main::m
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // call main::compute (sequential, local only)
    let _result = compute(*guard);
    // write_shared main::acc = 1 (still holding the mutex)
    *guard = 1;
    // mutex_unlock main::m
    drop(guard);
}

fn main() { crate::cir_trace::init();
    // Shared resources: m guards acc, acc init 0
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#926", 0));

    // scope { w1, w2 }
    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#1002", move || {
        w1(&m1);
    });
    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#1093", move || {
        w2(&m2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // After joins, read shared state only to print the terminal line.
    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
 crate::cir_trace::finish();}
