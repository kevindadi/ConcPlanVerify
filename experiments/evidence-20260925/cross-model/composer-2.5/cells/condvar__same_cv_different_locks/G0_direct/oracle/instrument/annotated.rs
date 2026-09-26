mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Condvar>, announce: concir_sync::Permit) {
    let mut g = m1.lock().unwrap();
    announce.release();
    while !*g {
        g = cv.wait(g).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Condvar>, announce: concir_sync::Permit) {
    let mut g = m2.lock().unwrap();
    announce.release();
    while !*g {
        g = cv.wait(g).unwrap();
    }
}

fn notifier(
    ready: Arc<Semaphore>,
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Condvar>,
) {
    let _p1 = ready.acquire();
    let _p2 = ready.acquire();
    {
        let mut g1 = m1.lock().unwrap();
        *g1 = true;
        let mut g2 = m2.lock().unwrap();
        *g2 = true;
    }
    let _g1 = m1.lock().unwrap();
    let _g2 = m2.lock().unwrap();
    cv.notify_all();
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let ready = Semaphore::new_named("ready_semaphore0", 2);

    let a1 = ready.acquire();
    let a2 = ready.acquire();

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let w1_handle = cir_trace::spawn("w1", move || w1(m1_w1, cv_w1, a1));

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let w2_handle = cir_trace::spawn("w2", move || w2(m2_w2, cv_w2, a2));

    let ready_n = Arc::clone(&ready);
    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier", move || notifier(ready_n, m1_n, m2_n, cv_n));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
