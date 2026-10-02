use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let shared = Arc::new(Shared {
        m1: Mutex::new(WaiterState {
            announced: false,
            notified: false,
        }),
        m2: Mutex::new(WaiterState {
            announced: false,
            notified: false,
        }),
        cv: Condvar::new(),
        ready: Mutex::new(0),
        ready_cv: Condvar::new(),
    });

    let s1 = Arc::clone(&shared);
    let w1 = thread::spawn(move || {
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
    let w2 = thread::spawn(move || {
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
    let notifier = thread::spawn(move || {
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
}
