use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Condvar>, announce: impl FnOnce()) {
    let mut guard = m1.lock().unwrap();
    announce();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Condvar>, announce: impl FnOnce()) {
    let mut guard = m2.lock().unwrap();
    announce();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
    ready: Arc<Semaphore>,
) {
    let arrived_w1 = ready.acquire();
    let arrived_w2 = ready.acquire();
    drop(arrived_w1);
    drop(arrived_w2);

    let mut g1 = m1.lock().unwrap();
    let mut g2 = m2.lock().unwrap();
    *g1 = true;
    *g2 = true;
    cv.notify_all();
    drop(g2);
    drop(g1);
}

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let ready = Semaphore::new(2);

    let permit_w1 = ready.acquire();
    let permit_w2 = ready.acquire();

    let m1_w = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let t_w1 = thread::spawn(move || {
        w1(m1_w, cv_w1, move || {
            permit_w1.release();
        });
    });

    let m2_w = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let t_w2 = thread::spawn(move || {
        w2(m2_w, cv_w2, move || {
            permit_w2.release();
        });
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);
    let t_notifier = thread::spawn(move || {
        notifier(m1_n, m2_n, cv_n, ready_n);
    });

    t_w1.join().unwrap();
    t_w2.join().unwrap();
    t_notifier.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
}
