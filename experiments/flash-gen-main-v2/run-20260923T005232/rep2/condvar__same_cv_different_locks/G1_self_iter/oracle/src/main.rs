mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));   // paired with m1
    let cv2 = Arc::new(Condvar::new_named("cv2_condvar0"));  // paired with m2
    let ready = Arc::new((Mutex::new_named("ready_mutex0", 0usize), Condvar::new_named("ready_condvar0")));

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);
    let w1 = cir_trace::spawn("w1", move || {
        let mut guard = m1_w1.lock().unwrap();
        // Announce we are about to wait.
        {
            let (lock, cvar) = &*ready_w1;
            let mut count = lock.lock().unwrap();
            *count += 1;
            cvar.notify_all();
        }
        // Wait while holding m1.
        while !*guard {
            guard = cv_w1.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv2);
    let ready_w2 = Arc::clone(&ready);
    let w2 = cir_trace::spawn("w2", move || {
        let mut guard = m2_w2.lock().unwrap();
        {
            let (lock, cvar) = &*ready_w2;
            let mut count = lock.lock().unwrap();
            *count += 1;
            cvar.notify_all();
        }
        while !*guard {
            guard = cv_w2.wait(guard).unwrap();
        }
        *guard = true;
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let cv2_n = Arc::clone(&cv2);
    let ready_n = Arc::clone(&ready);
    let notifier = cir_trace::spawn("notifier", move || {
        // Wait until both waiters have announced.
        {
            let (lock, cvar) = &*ready_n;
            let mut count = lock.lock().unwrap();
            while *count < 2 {
                count = cvar.wait(count).unwrap();
            }
        }
        // Wake each waiter while holding its lock.
        {
            let mut g1 = m1_n.lock().unwrap();
            *g1 = true;
            cv_n.notify_all();
        }
        {
            let mut g2 = m2_n.lock().unwrap();
            *g2 = true;
            cv2_n.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
