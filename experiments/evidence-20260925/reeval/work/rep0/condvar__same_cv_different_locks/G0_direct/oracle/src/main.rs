mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct WaiterState {
    announced: bool,
    notified: bool,
}

struct Shared {
    m1: Mutex<WaiterState>,
    m2: Mutex<WaiterState>,
    cv: Condvar,
    ready: Mutex<usize>,
    ready_cv: Condvar,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m1: Mutex::new_named("shared_mutex0", WaiterState {
            announced: false,
            notified: false,
        }),
        m2: Mutex::new_named("shared_mutex1", WaiterState {
            announced: false,
            notified: false,
        }),
        cv: Condvar::new_named("shared_condvar0"),
        ready: Mutex::new_named("shared_mutex2", 0),
        ready_cv: Condvar::new_named("shared_condvar1"),
    });

    let s1 = Arc::clone(&shared);
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = s1.m1.lock().unwrap();
        guard.announced = true;
        {
            let mut r = s1.ready.lock().unwrap();
            *r += 1;
            s1.ready_cv.notify_all();
        }
        while !guard.notified {
            guard = s1.cv.wait(guard).unwrap();
        }
    });

    let s2 = Arc::clone(&shared);
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = s2.m2.lock().unwrap();
        guard.announced = true;
        {
            let mut r = s2.ready.lock().unwrap();
            *r += 1;
            s2.ready_cv.notify_all();
        }
        while !guard.notified {
            guard = s2.cv.wait(guard).unwrap();
        }
    });

    let s3 = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        {
            let mut r = s3.ready.lock().unwrap();
            while *r < 2 {
                r = s3.ready_cv.wait(r).unwrap();
            }
        }
        {
            let mut g1 = s3.m1.lock().unwrap();
            g1.notified = true;
            s3.cv.notify_all();
        }
        {
            let mut g2 = s3.m2.lock().unwrap();
            g2.notified = true;
            s3.cv.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
