use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared state: number of waiters that have announced they are about to wait.
    let announced = Arc::new((Mutex::new(0usize), Condvar::new()));

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));

    // Shared condition variable both waiters block on.
    let cond = Arc::new(Condvar::new());

    // Flag set by notifier to tell waiters they may proceed.
    let go = Arc::new((Mutex::new(false), Condvar::new()));

    let announced_w = Arc::clone(&announced);
    let lock1_w = Arc::clone(&lock1);
    let cond_w = Arc::clone(&cond);
    let go_w = Arc::clone(&go);

    let waiter1 = thread::spawn(move || {
        // Hold own lock while waiting on the shared condition variable.
        let mut guard = lock1_w.lock().unwrap();

        // Announce that we are about to wait.
        {
            let (m, cv) = &*announced_w;
            let mut count = m.lock().unwrap();
            *count += 1;
            cv.notify_all();
        }

        // Wait on the shared condition variable while holding our own lock.
        while !*go_w.0.lock().unwrap() {
            guard = cond_w.wait(guard).unwrap();
        }
        // Lock released here when guard drops.
    });

    let announced_w2 = Arc::clone(&announced);
    let lock2_w = Arc::clone(&lock2);
    let cond_w2 = Arc::clone(&cond);
    let go_w2 = Arc::clone(&go);

    let waiter2 = thread::spawn(move || {
        let mut guard = lock2_w.lock().unwrap();

        {
            let (m, cv) = &*announced_w2;
            let mut count = m.lock().unwrap();
            *count += 1;
            cv.notify_all();
        }

        while !*go_w2.0.lock().unwrap() {
            guard = cond_w2.wait(guard).unwrap();
        }
    });

    // Notifier: wait until both waiters have announced themselves.
    {
        let (m, cv) = &*announced;
        let mut count = m.lock().unwrap();
        while *count < 2 {
            count = cv.wait(count).unwrap();
        }
    }

    // Wake the waiters while holding each lock that a waiter needs.
    {
        let _g1 = lock1.lock().unwrap();
        let _g2 = lock2.lock().unwrap();

        {
            let (m, _cv) = &*go;
            let mut flag = m.lock().unwrap();
            *flag = true;
        }

        // Notify on the shared condition variable while holding both locks.
        cond.notify_all();
    }

    waiter1.join().unwrap();
    waiter2.join().unwrap();

    println!("DONE done=1");
}
