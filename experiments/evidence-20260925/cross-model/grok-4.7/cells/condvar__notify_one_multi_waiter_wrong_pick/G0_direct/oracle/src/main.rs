mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

struct Shared {
    go: bool,
    waiters: usize,
}

fn w1(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, ready: impl FnOnce(), finished: impl FnOnce()) {
    let mut guard = m.lock().unwrap();
    ready();
    while !guard.go {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
    drop(guard);
    finished();
}

fn w2(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, ready: impl FnOnce(), finished: impl FnOnce()) {
    let mut guard = m.lock().unwrap();
    ready();
    while !guard.go {
        guard = cv.wait(guard).unwrap();
    }
    guard.waiters -= 1;
    drop(guard);
    finished();
}

fn notifier(m: Arc<Mutex<Shared>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let _ready_a = g12.acquire();
    let _ready_b = g12.acquire();
    {
        let mut guard = m.lock().unwrap();
        guard.go = true;
        cv.notify_all();
    }
    let _done_a = gN.acquire();
    let _done_b = gN.acquire();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared {
        go: false,
        waiters: 2,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // Permits start taken, so each release is a one-way signal.
    // g12: both waiters are holding `m` and about to wait.
    // gN: both waiters have left the wait.
    let g12 = Semaphore::new_named("g12_semaphore0", 2);
    let gN = Semaphore::new_named("gN_semaphore0", 2);
    let ready_w1 = g12.acquire();
    let ready_w2 = g12.acquire();
    let done_w1 = gN.acquire();
    let done_w2 = gN.acquire();

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let mn = Arc::clone(&m);
    let cvn = Arc::clone(&cv);

    let h1 = cir_trace::spawn("w1", move || {
        w1(
            m1,
            cv1,
            move || ready_w1.release(),
            move || done_w1.release(),
        );
    });
    let h2 = cir_trace::spawn("w2", move || {
        w2(
            m2,
            cv2,
            move || ready_w2.release(),
            move || done_w2.release(),
        );
    });
    let hn = cir_trace::spawn("notifier", move || {
        notifier(mn, cvn, g12, gN);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();

    let waiters = m.lock().unwrap().waiters;
    println!("DONE waiters={waiters}");
 cir_trace::finish();}
