mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: &Arc<Mutex<i32>>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // read_shared main::c
    let c = *guard;
    // write_shared main::c (c + 1)
    *guard = c + 1;
    // mutex_unlock main::m (guard dropped)
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // read_shared main::c
    let c = *guard;
    // write_shared main::c (c + 1)
    *guard = c + 1;
    // mutex_unlock main::m (guard dropped)
    drop(guard);
}

fn main() { crate::cir_trace::init();
    // shared resource m guarding c (c starts at 0, ranges 0..=2)
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#692", 0));

    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#744", move || w1(&m1));

    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#819", move || w2(&m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap() / 2;
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
