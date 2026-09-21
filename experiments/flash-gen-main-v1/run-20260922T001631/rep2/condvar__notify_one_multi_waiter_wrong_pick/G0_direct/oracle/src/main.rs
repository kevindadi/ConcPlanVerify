mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    // Number of waiters that have reached the waiting point.
    ready: usize,
    // Number of waiters that have been released.
    released: usize,
    // Set to true once the notifier has signalled.
    notified: bool,
}

fn main() { cir_trace::init();
    let shared = Arc::new((
        Mutex::new_named("shared_mutex0", Shared {
            ready: 0,
            released: 0,
            notified: false,
        }),
        Condvar::new_named("shared_condvar0"),
    ));

    let mut handles = Vec::new();

    // Two waiters.
    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();

            // Mark this waiter as ready.
            guard.ready += 1;
            cvar.notify_all();

            // Wait until the notifier has signalled.
            while !guard.notified {
                guard = cvar.wait(guard).unwrap();
            }

            guard.released += 1;
        }));
    }

    // Notifier.
    {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();

            // Wait until both waiters are ready.
            while guard.ready < 2 {
                guard = cvar.wait(guard).unwrap();
            }

            // Wake every blocked waiter.
            guard.notified = true;
            cvar.notify_all();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let (lock, _) = &*shared;
    let guard = lock.lock().unwrap();
    let remaining = 2 - guard.released;
    println!("DONE waiters={}", remaining);
 cir_trace::finish();}
