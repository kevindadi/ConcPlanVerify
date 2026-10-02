mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// compute: pure local computation (no shared state).
fn compute(n: i64) -> i64 {
    let mut tmp = 0;
    let mut res = 0;
    tmp = n;
    res = tmp + 1;
    res
}

fn w1(m: &Mutex<i64>) {
    // mutex_lock {m}
    let mut guard = m.lock().unwrap();
    // call compute(1)
    let _r = compute(1);
    // write_shared {acc} <- 1
    *guard = 1;
    // mutex_unlock {m}: guard drops at end of scope, release before returning
    drop(guard);
}

fn w2(m: &Mutex<i64>) {
    // mutex_lock {m}
    let mut guard = m.lock().unwrap();
    // call compute(1)
    let _r = compute(1);
    // write_shared {acc} <- 1
    *guard = 1;
    // mutex_unlock {m}: guard drops at end of scope, release before returning
    drop(guard);
}

fn main() { cir_trace::init();
    // Resource m (Mutex), protecting shared variable acc (init 0).
    let m: Arc<Mutex<i64>> = Arc::new(Mutex::new_named("res_mutex0#897", 0));

    // scope {funcs: [w1, w2]}: start the two workers.
    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#1004", move || w1(&m1));
    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#1078", move || w2(&m2));

    // Join every spawned thread.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // After joins, read shared state only to print the terminal line.
    let acc = *m.lock().unwrap();
    println!("DONE done={}", acc);
 cir_trace::finish();}
