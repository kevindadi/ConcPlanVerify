mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // m1 and m2 are the two waiters' own locks.
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", false));
    // cv is the shared condition variable used with each waiter's lock.
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    // ready counts how many waiters have announced themselves.
    let ready = Semaphore::new_named("ready_semaphore0", 0);

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ready_w2 = Arc::clone(&ready);

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);

    // w1: hold m1, announce, wait for notification while holding m1.
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m1_w1.lock().unwrap();
        // Announce that we are about to wait.
        ready_w1.acquire().release();
        // Wait for the notification while holding m1.
        while !*guard {
            guard = cv_w1.wait(guard).unwrap();
        }
    });

    // w2: hold m2, announce, wait for notification while holding m2.
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m2_w2.lock().unwrap();
        ready_w2.acquire().release();
        while !*guard {
            guard = cv_w2.wait(guard).unwrap();
        }
    });

    // notifier: wait until both waiters have announced, then wake them
    // while holding each lock.
    let notifier = cir_trace::spawn("notifier", move || {
        // Wait for both announcements.
        ready_n.acquire().release();
        ready_n.acquire().release();

        // Wake w1 while holding m1.
        {
            let mut g1 = m1_n.lock().unwrap();
            *g1 = true;
            cv_n.notify_all();
        }

        // Wake w2 while holding m2.
        {
            let mut g2 = m2_n.lock().unwrap();
            *g2 = true;
            cv_n.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
