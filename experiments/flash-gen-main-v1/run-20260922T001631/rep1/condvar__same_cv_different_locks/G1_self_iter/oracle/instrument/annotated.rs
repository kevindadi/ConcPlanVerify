mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared condition variable for both waiters.
    let condvar = Arc::new(Condvar::new_named("condvar_condvar0"));

    // Two different locks, one per waiter.
    let lock1 = Arc::new(Mutex::new_named("lock1_mutex0", false));
    let lock2 = Arc::new(Mutex::new_named("lock2_mutex0", false));

    // Counter of waiters that have announced they are about to wait.
    let announced = Arc::new((Mutex::new_named("announced_mutex0", 0usize), Condvar::new_named("announced_condvar0")));

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
            // Wait on shared condvar while holding own lock.
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
            // Wait on shared condvar while holding own lock.
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
            // Wait until both waiters have announced.
            {
                let (m, c) = &*announced;
                let mut count = m.lock().unwrap();
                while *count < 2 {
                    count = c.wait(count).unwrap();
                }
            }

            // Hold each lock while waking, so waiters can wake and finish.
            let mut g1 = lock1.lock().unwrap();
            let mut g2 = lock2.lock().unwrap();
            *g1 = true;
            *g2 = true;
            condvar.notify_all();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
