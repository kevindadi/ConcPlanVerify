mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", ()));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let ready = Semaphore::new_named("ready_semaphore0", 2);
    let notified = Arc::new(AtomicBool::new(false));

    let mut ann_w1 = ready.acquire();
    let mut ann_w2 = ready.acquire();

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let notified_w1 = Arc::clone(&notified);
    let w1 = cir_trace::spawn("w1", move || {
        let mut g = m1_w1.lock().unwrap();
        ann_w1.release();
        while !notified_w1.load(Ordering::Acquire) {
            g = cv_w1.wait(g).unwrap();
        }
    });

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let notified_w2 = Arc::clone(&notified);
    let w2 = cir_trace::spawn("w2", move || {
        let mut g = m2_w2.lock().unwrap();
        ann_w2.release();
        while !notified_w2.load(Ordering::Acquire) {
            g = cv_w2.wait(g).unwrap();
        }
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);
    let notified_n = Arc::clone(&notified);
    let notifier = cir_trace::spawn("notifier", move || {
        let _p1 = ready_n.acquire();
        let _p2 = ready_n.acquire();
        let g1 = m1_n.lock().unwrap();
        let g2 = m2_n.lock().unwrap();
        notified_n.store(true, Ordering::Release);
        cv_n.notify_all();
        drop(g2);
        drop(g1);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
