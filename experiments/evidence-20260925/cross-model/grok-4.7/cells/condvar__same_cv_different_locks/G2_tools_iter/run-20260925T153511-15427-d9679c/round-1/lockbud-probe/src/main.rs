use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
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

fn notifier(m1: &Mutex<bool>, m2: &Mutex<bool>, cv: &Condvar, ready: &Arc<Semaphore>) {
    let _a = ready.acquire();
    let _b = ready.acquire();
    {
        let mut g1 = m1.lock().unwrap();
        let mut g2 = m2.lock().unwrap();
        *g1 = true;
        *g2 = true;
        cv.notify_all();
    }
}

fn main() {
    let m1 = Mutex::new(false);
    let m2 = Mutex::new(false);
    let cv = Condvar::new();
    let ready = Semaphore::new(2);
    let permit1 = ready.acquire();
    let permit2 = ready.acquire();

    thread::scope(|scope| {
        scope.spawn(|| {
            w1(&m1, &cv, || permit1.release());
        });
        scope.spawn(|| {
            w2(&m2, &cv, || permit2.release());
        });
        scope.spawn(|| {
            notifier(&m1, &m2, &cv, &ready);
        });
    });

    println!("DONE done=1");
}
