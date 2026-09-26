mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::Arc;
use std::thread;

use concir_sync::{Condvar, Mutex, Semaphore};

struct Shared {
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    gN: Arc<Semaphore>,
}

fn w1(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.signal();
    let guard = shared.cv.wait(guard).unwrap();
    shared.gN.wait();
    drop(guard);
}

fn w2(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.signal();
    let guard = shared.cv.wait(guard).unwrap();
    shared.gN.wait();
    drop(guard);
}

fn notifier(shared: Arc<Shared>) {
    shared.g12.wait();
    shared.g12.wait();
    let guard = shared.m.lock().unwrap();
    shared.cv.notify_all();
    shared.gN.signal();
    shared.gN.signal();
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("main::m", ()),
        cv: Condvar::new_named("main::cv"),
        g12: Semaphore::new_named("main::g12", 0),
        gN: Semaphore::new_named("main::gN", 0),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));
    let h3 = cir_trace::spawn("notifier", move || notifier(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
