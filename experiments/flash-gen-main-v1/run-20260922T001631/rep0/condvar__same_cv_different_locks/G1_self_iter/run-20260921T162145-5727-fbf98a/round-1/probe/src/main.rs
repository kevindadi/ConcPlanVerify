use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared condition variable used by both waiters.
    let condvar = Arc::new(Condvar::new());

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new(false));
    let lock2 = Arc::new(Mutex::new(false));

    // Counter of waiters that have announced they are about to wait.
    let announced = Arc::new((Mutex::new(0usize), Condvar::new()));

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
    let waiter1 = thread::spawn(move || {
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
    let waiter2 = thread::spawn(move || {
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
    let notifier = thread::spawn(move || {
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
}
