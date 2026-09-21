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

    let mut handles = Vec::new();

    // Waiter 1
    {
        let condvar = Arc::clone(&condvar);
        let lock1 = Arc::clone(&lock1);
        let announced = Arc::clone(&announced);
        handles.push(thread::spawn(move || {
            let mut guard = lock1.lock().unwrap();
            // Announce before waiting.
            {
                let (m, c) = &*announced;
                let mut count = m.lock().unwrap();
                *count += 1;
                c.notify_all();
            }
            // Wait on the shared condvar while holding our own lock.
            while !*guard {
                guard = condvar.wait(guard).unwrap();
            }
        }));
    }

    // Waiter 2
    {
        let condvar = Arc::clone(&condvar);
        let lock2 = Arc::clone(&lock2);
        let announced = Arc::clone(&announced);
        handles.push(thread::spawn(move || {
            let mut guard = lock2.lock().unwrap();
            // Announce before waiting.
            {
                let (m, c) = &*announced;
                let mut count = m.lock().unwrap();
                *count += 1;
                c.notify_all();
            }
            // Wait on the shared condvar while holding our own lock.
            while !*guard {
                guard = condvar.wait(guard).unwrap();
            }
        }));
    }

    // Notifier
    {
        let condvar = Arc::clone(&condvar);
        let lock1 = Arc::clone(&lock1);
        let lock2 = Arc::clone(&lock2);
        let announced = Arc::clone(&announced);
        handles.push(thread::spawn(move || {
            // Wait until both waiters have announced themselves.
            {
                let (m, c) = &*announced;
                let mut count = m.lock().unwrap();
                while *count < 2 {
                    count = c.wait(count).unwrap();
                }
            }

            // Acquire both locks before waking, so each waiter can wake and finish.
            let mut g1 = lock1.lock().unwrap();
            let mut g2 = lock2.lock().unwrap();

            *g1 = true;
            *g2 = true;

            // Wake all waiters while holding the locks.
            condvar.notify_all();

            // Release locks after waking.
            drop(g1);
            drop(g2);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
