mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared state: number of waiters that have announced they are about to wait.
    let announced = Arc::new((Mutex::new_named("announced_mutex0", 0usize), Condvar::new_named("announced_condvar0")));

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", ()));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", ()));

    // Shared condition variable both waiters block on.
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // Shared predicate, read by waiters while holding their own lock and
    // written by the notifier while holding both locks.
    let go = Arc::new(AtomicBool::new(false));

    let announced_w = Arc::clone(&announced);
    let lock1_w = Arc::clone(&lock1);
    let cv_w = Arc::clone(&cv);
    let go_w = Arc::clone(&go);

    let waiter1 = cir_trace::spawn("waiter1", move || {
        // Hold own lock while waiting on the shared condition variable.
        let mut guard = lock1_w.lock().unwrap();

        // Announce that we are about to wait.
        {
            let (m, c) = &*announced_w;
            let mut count = m.lock().unwrap();
            *count += 1;
            c.notify_all();
        }

        // Wait on the shared condition variable while holding our own lock.
        // `cv.wait` atomically releases `guard` (our own lock) while blocked,
        // so the notifier can acquire it to wake us.
        while !go_w.load(Ordering::SeqCst) {
            guard = cv_w.wait(guard).unwrap();
        }
    });

    let announced_w2 = Arc::clone(&announced);
    let lock2_w = Arc::clone(&lock2);
    let cv_w2 = Arc::clone(&cv);
    let go_w2 = Arc::clone(&go);

    let waiter2 = cir_trace::spawn("waiter2", move || {
        let mut guard = lock2_w.lock().unwrap();

        {
            let (m, c) = &*announced_w2;
            let mut count = m.lock().unwrap();
            *count += 1;
            c.notify_all();
        }

        while !go_w2.load(Ordering::SeqCst) {
            guard = cv_w2.wait(guard).unwrap();
        }
    });

    // Notifier: wait until both waiters have announced themselves.
    {
        let (m, c) = &*announced;
        let mut count = m.lock().unwrap();
        while *count < 2 {
            count = c.wait(count).unwrap();
        }
    }

    // Wake the waiters while holding each lock that a waiter needs.
    // Acquiring these locks can only succeed once each waiter has released
    // its own lock inside `cv.wait`, so both waiters are guaranteed to be
    // blocked on the shared condvar when we notify.
    {
        let _g1 = lock1.lock().unwrap();
        let _g2 = lock2.lock().unwrap();

        go.store(true, Ordering::SeqCst);
        cv.notify_all();
    }

    waiter1.join().unwrap();
    waiter2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
