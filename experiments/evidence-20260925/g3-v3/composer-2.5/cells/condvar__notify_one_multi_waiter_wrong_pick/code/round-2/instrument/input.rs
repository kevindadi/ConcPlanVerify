use std::sync::Arc;
use std::thread;

use concir_sync::{Condvar, Mutex, Semaphore};

struct Shared {
    m: Mutex<()>,
    cv: Condvar,
    g12: Semaphore,
    gN: Semaphore,
}

fn w1(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.release();
    let guard = shared.cv.wait(guard).unwrap();
    shared.gN.acquire();
    drop(guard);
}

fn w2(shared: Arc<Shared>) {
    let guard = shared.m.lock().unwrap();
    shared.g12.release();
    let guard = shared.cv.wait(guard).unwrap();
    shared.gN.acquire();
    drop(guard);
}

fn notifier(shared: Arc<Shared>) {
    shared.g12.acquire();
    shared.g12.acquire();
    let guard = shared.m.lock().unwrap();
    shared.cv.notify_all();
    shared.gN.release();
    shared.gN.release();
    drop(guard);
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        cv: Condvar::new(),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));
    let h3 = thread::spawn(move || notifier(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE waiters=0");
}
