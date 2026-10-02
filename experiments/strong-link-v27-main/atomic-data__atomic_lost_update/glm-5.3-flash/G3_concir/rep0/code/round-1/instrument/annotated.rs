mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    c: i32,
}

fn w1(m: &Mutex<State>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // write_shared main::c : c + 1  (read-modify-write, indivisible under the lock)
    let cur = guard.c;
    guard.c = cur + 1;
    // mutex_unlock main::m
    drop(guard);
}

fn w2(m: &Mutex<State>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();
    // write_shared main::c : c + 1  (read-modify-write, indivisible under the lock)
    let cur = guard.c;
    guard.c = cur + 1;
    // mutex_unlock main::m
    drop(guard);
}

fn main() { cir_trace::init();
    // shared resources: m (Mutex) protecting c (var, init 0)
    let m: Arc<Mutex<State>> = Arc::new(Mutex::new_named("res_mutex0#738", State { c: 0 }));

    // spawn main::w1 -> h1
    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#831", move || {
        w1(&m1);
    });

    // spawn main::w2 -> h2
    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#951", move || {
        w2(&m2);
    });

    // join h1
    h1.join().unwrap();
    // join h2
    h2.join().unwrap();

    // terminal line
    println!("DONE done=1");
 cir_trace::finish();}
