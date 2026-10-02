mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<()>,
    acc: AtomicU64,
}

fn compute() -> u64 {
    (1..=100).sum()
}

fn w1(shared: Arc<Shared>) {
    let guard = shared.m.lock().expect("mutex poisoned");
    let local = compute();
    shared.acc.fetch_add(local, Ordering::Relaxed);
    drop(guard);
}

fn w2(shared: Arc<Shared>) {
    let guard = shared.m.lock().expect("mutex poisoned");
    let local = compute();
    shared.acc.fetch_add(local, Ordering::Relaxed);
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("m#632", ()),
        acc: AtomicU64::new(0),
    });

    let w1_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w1#754", move || w1(shared))
    };
    let w2_handle = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w2#867", move || w2(shared))
    };

    w1_handle.join().expect("w1 panicked");
    w2_handle.join().expect("w2 panicked");

    assert_eq!(shared.acc.load(Ordering::Relaxed), 10_100);
    println!("DONE done=1");
 cir_trace::finish();}
