use concir_sync::Semaphore;
use std::sync::{Condvar, Mutex};
use std::thread;

fn w1(m1: &Mutex<bool>, cv: &Condvar, announce: impl FnOnce()) {
    let mut guard = m1.lock().unwrap();
    announce();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn w2(m2: &Mutex<bool>, cv: &Condvar, announce: impl FnOnce()) {
    let mut guard = m2.lock().unwrap();
    announce();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(
    m1: &Mutex<bool>,
    m2: &Mutex<bool>,
    cv1: &Condvar,
    cv2: &Condvar,
    ready: &Semaphore,
) {
    let _ann_a = ready.acquire();
    let _ann_b = ready.acquire();
    {
        let mut g1 = m1.lock().unwrap();
        *g1 = true;
        cv1.notify_all();
    }
    {
        let mut g2 = m2.lock().unwrap();
        *g2 = true;
        cv2.notify_all();
    }
}

fn main() {
    let m1 = Mutex::new(false);
    let m2 = Mutex::new(false);
    let cv = Condvar::new();
    let cv2 = Condvar::new();
    let ready = Semaphore::new(2);
    let ann1 = ready.acquire();
    let ann2 = ready.acquire();

    thread::scope(|scope| {
        scope.spawn(|| {
            w1(&m1, &cv, || ann1.release());
        });
        scope.spawn(|| {
            w2(&m2, &cv2, || ann2.release());
        });
        scope.spawn(|| {
            notifier(&m1, &m2, &cv, &cv2, &ready);
        });
    });

    println!("DONE done=1");
}
