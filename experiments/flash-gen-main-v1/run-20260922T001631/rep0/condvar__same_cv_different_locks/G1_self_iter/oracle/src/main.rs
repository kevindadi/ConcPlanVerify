mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared condition variable used by both waiters.
    let condvar = Arc::new(Condvar::new_named("condvar_condvar0"));

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", false));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", false));

    // Counter of waiters that have announced they are about to wait.
    let announced = Arc::new((Mutex::new_named("announced_mutex0", 0usize), Condvar::new_named("announced_condvar0")));

    let c1 = Arc::clone(&condvar);
    let l1 = Arc::clone(&lock1);
    let a1 = Arc::clone(&announced);

    let c2 = Arc::clone(&condvar);
    let l2 = Arc::clone(&lock2);
    let a2 = Arc::clone(&announced);

    let cn = Arc::clone(&condvar);
    let ln1 = Arc::clone(&lock1);
    let ln2 = Arc::clone(&lock2);
    let an = Arc::clone(&announced);

    // Waiter 1
    let waiter1 = cir_trace::spawn("waiter1", move || {
        let mut guard = l1.lock().unwrap();
        // Announce that we are about to wait.
        {
            let (m, cv) = &*a1;
            let mut count = m.lock().unwrap();
            *count += 1;
            cv.notify_all();
        }
        // Wait on the shared condvar while holding our own lock.
        while !*guard {
            guard = c1.wait(guard).unwrap();
        }
    });

    // Waiter 2
    let waiter2 = cir_trace::spawn("waiter2", move || {
        let mut guard = l2.lock().unwrap();
        {
            let (m, cv) = &*a2;
            let mut count = m.lock().unwrap();
            *count += 1;
            cv.notify_all();
        }
        while !*guard {
            guard = c2.wait(guard).unwrap();
        }
    });

    // Notifier
    let notifier = cir_trace::spawn("notifier", move || {
        // Wait until both waiters have announced themselves.
        {
            let (m, cv) = &*an;
            let mut count = m.lock().unwrap();
            while *count < 2 {
                count = cv.wait(count).unwrap();
            }
        }

        // Wake the waiters while holding each lock they need.
        {
            let mut g1 = ln1.lock().unwrap();
            *g1 = true;
            cn.notify_all();
        }
        {
            let mut g2 = ln2.lock().unwrap();
            *g2 = true;
            cn.notify_all();
        }
    });

    waiter1.join().unwrap();
    waiter2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
