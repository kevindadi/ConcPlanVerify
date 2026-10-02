mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Shared resources:
//   m: Mutex (sync resource)
//   c: var (Int, init 0, declared range 0..=2), protected by m.
// c is stored directly as the primitive guarded by m (Mutex<i32>),
// so every read/write of c happens while holding m.

fn w1(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // write_shared c = c + 1 (c lives inside m; guard held during read+write)
    *guard = *guard + 1;
    // mutex_unlock m (guard drops)
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // write_shared c = c + 1 (c lives inside m; guard held during read+write)
    *guard = *guard + 1;
    // mutex_unlock m (guard drops)
    drop(guard);
}

fn main() { cir_trace::init();
    // shared resource m (Mutex) guarding c; c starts at 0, declared range 0..=2
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#919", 0));

    // spawn h1: main::w1
    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#997", move || w1(&m1));

    // spawn h2: main::w2
    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#1098", move || w2(&m2));

    // join h1
    h1.join().unwrap();
    // join h2
    h2.join().unwrap();

    // After joins: read shared state only to print the terminal line.
    // c is within its declared range 0..=2 in every reachable state.
    let _final_c = m.lock().unwrap();

    // terminal line
    println!("DONE done=1");
 cir_trace::finish();}
