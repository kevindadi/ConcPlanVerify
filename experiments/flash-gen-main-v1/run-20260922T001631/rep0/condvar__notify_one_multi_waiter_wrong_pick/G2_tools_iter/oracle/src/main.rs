mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let pair = Arc::new((Mutex::new_named("pair_mutex0", 0usize), Condvar::new_named("pair_condvar0")));
    let ready = Arc::new((Mutex::new_named("ready_mutex0", 0usize), Condvar::new_named("ready_condvar0")));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let pair = Arc::clone(&pair);
        let ready = Arc::clone(&ready);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*pair;
            let (rlock, rcvar) = &*ready;

            // Signal readiness.
            {
                let mut r = rlock.lock().unwrap();
                *r += 1;
                rcvar.notify_all();
            }

            // Wait until notifier says go.
            let mut guard = lock.lock().unwrap();
            while *guard == 0 {
                guard = cvar.wait(guard).unwrap();
            }
        }));
    }

    // Notifier thread.
    let pair_n = Arc::clone(&pair);
    let ready_n = Arc::clone(&ready);
    handles.push(thread::spawn(move || {
        let (lock, cvar) = &*pair_n;
        let (rlock, rcvar) = &*ready_n;

        // Wait until both waiters are ready.
        let mut r = rlock.lock().unwrap();
        while *r < 2 {
            r = rcvar.wait(r).unwrap();
        }
        drop(r);

        // Take the lock, wake all waiters, release.
        let mut guard = lock.lock().unwrap();
        *guard = 1;
        cvar.notify_all();
        drop(guard);
    }));

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE waiters=0");
 cir_trace::finish();}
