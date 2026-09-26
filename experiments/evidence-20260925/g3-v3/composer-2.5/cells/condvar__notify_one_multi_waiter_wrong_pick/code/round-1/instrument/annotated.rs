mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, Semaphore};
use std::thread;

struct Shared {
    m: Mutex<()>,
    cv: Condvar,
    g12: Semaphore,
    gN: Semaphore,
}

fn w1(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.add_permits(1);
    let guard = shared.cv.wait(guard).unwrap();
    let _permit = shared.gN.acquire().unwrap();
    drop(guard);
}

fn w2(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.add_permits(1);
    let guard = shared.cv.wait(guard).unwrap();
    let _permit = shared.gN.acquire().unwrap();
    drop(guard);
}

fn notifier(shared: Arc<Shared>) {
    let _ = shared.g12.acquire();
    let _ = shared.g12.acquire();
    let guard = shared.m.lock().unwrap();
    shared.cv.notify_all();
    shared.gN.add_permits(1);
    shared.gN.add_permits(1);
    drop(guard);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        cv: Condvar::new_named("shared_condvar0"),
        g12: Semaphore::new_named("shared_semaphore0", 0),
        gN: Semaphore::new_named("shared_semaphore1", 0),
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
