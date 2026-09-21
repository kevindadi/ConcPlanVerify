mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Two different locks, one per waiter.
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", false));

    // Shared condition variable. Each waiter waits on it while holding its own lock.
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // Announcement counter: notifier waits until both waiters have announced.
    let announced = Arc::new((Mutex::new_named("announced_mutex0", 0usize), Condvar::new_named("announced_condvar0")));

    let m1_w = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ann_w1 = Arc::clone(&announced);

    let w1 = cir_trace::spawn("w1", move || {
        // Hold our own lock across announce + check + wait so the notifier
        // cannot set the flag until we are committed to waiting.
        let mut g = m1_w.lock().unwrap();
        {
            let (lock, cvar) = &*ann_w1;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        while !*g {
            g = cv_w1.wait(g).unwrap();
        }
    });

    let m2_w = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ann_w2 = Arc::clone(&announced);

    let w2 = cir_trace::spawn("w2", move || {
        let mut g = m2_w.lock().unwrap();
        {
            let (lock, cvar) = &*ann_w2;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        while !*g {
            g = cv_w2.wait(g).unwrap();
        }
    });

    // Notifier: wait until both waiters have announced, then wake them
    // while holding each lock the corresponding waiter needs.
    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ann_n = Arc::clone(&announced);

    let notifier = cir_trace::spawn("notifier", move || {
        // Wait until both waiters have announced.
        {
            let (lock, cvar) = &*ann_n;
            let mut n = lock.lock().unwrap();
            while *n < 2 {
                n = cvar.wait(n).unwrap();
            }
        }

        // Wake waiter 1 while holding m1. Because waiter 1 holds m1 across
        // its check-and-wait, acquiring m1 here guarantees waiter 1 is either
        // already waiting or will observe the flag without waiting.
        {
            let mut g1 = m1_n.lock().unwrap();
            *g1 = true;
            cv_n.notify_all();
        }

        // Wake waiter 2 while holding m2, with the same guarantee.
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
