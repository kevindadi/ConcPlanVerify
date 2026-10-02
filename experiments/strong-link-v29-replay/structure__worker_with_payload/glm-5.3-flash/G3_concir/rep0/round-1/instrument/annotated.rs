mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resources:
//   m   : the shared mutex (crate::cir_trace::sync::Mutex)
//   acc : shared variable protected by m, stored inside m's critical section
fn compute() -> i32 {
    // sequential helper: only local computation
    let tmp: i32 = 1;
    tmp
}

fn w1(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // call compute (local computation only, lock still held)
    let tmp = compute();
    // write_shared acc = 1 (acc lives under m, still holding the mutex)
    *guard = tmp;
    // mutex_unlock m (guard released here)
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // call compute (local computation only, lock still held)
    let tmp = compute();
    // write_shared acc = 1 (acc lives under m, still holding the mutex)
    *guard = tmp;
    // mutex_unlock m (guard released here)
    drop(guard);
}

fn main() { crate::cir_trace::init();
    // resource m guarding acc (init 0)
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#1054", 0));

    // spawn w1 (handle h1)
    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#1134", move || {
        w1(&m1);
    });

    // spawn w2 (handle h2)
    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#1254", move || {
        w2(&m2);
    });

    // join h1, join h2
    h1.join().unwrap();
    h2.join().unwrap();

    // after joins: read shared state only to print the terminal line
    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
 crate::cir_trace::finish();}
