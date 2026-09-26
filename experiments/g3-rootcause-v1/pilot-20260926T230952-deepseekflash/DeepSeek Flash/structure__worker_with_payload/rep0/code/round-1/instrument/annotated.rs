mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<i64>,
}

fn compute() -> i64 {
    let tmp = 1;
    tmp
}

fn w1(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    let _ = compute();
    let _ = *guard;
    *guard = *guard + 1;
    drop(guard);
}

fn w2(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    let _ = compute();
    let _ = *guard;
    *guard = *guard + 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared { m: Mutex::new_named("shared_mutex0", 0) });

    let s1 = Arc::clone(&shared);
    let t1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&shared);
    let t2 = cir_trace::spawn("w2", move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = *shared.m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
