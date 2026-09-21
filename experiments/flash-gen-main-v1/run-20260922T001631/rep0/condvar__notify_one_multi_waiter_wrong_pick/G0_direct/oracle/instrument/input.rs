use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    // Number of waiters that have reached the waiting point.
    ready: usize,
    // Number of waiters that have been released.
    released: usize,
    // Set to true once the notifier has signalled.
    notified: bool,
}

fn main() {
    let shared = Arc::new((
        Mutex::new(Shared {
            ready: 0,
            released: 0,
            notified: false,
        }),
        Condvar::new(),
    ));

    let num_waiters = 2;

    let mut handles = Vec::new();

    // Spawn waiters.
    for _ in 0..num_waiters {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();
            // R4: hold the lock while waiting.
            guard.ready += 1;
            // Wake the notifier if it is waiting for readiness.
            cvar.notify_all();
            while !guard.notified {
                guard = cvar.wait(guard).unwrap();
            }
            guard.released += 1;
        }));
    }

    // Spawn notifier.
    {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();
            // R6: wait until both waiters are ready.
            while guard.ready < num_waiters {
                guard = cvar.wait(guard).unwrap();
            }
            // R5: hold the lock while waking.
            guard.notified = true;
            // R7: wake every blocked waiter.
            cvar.notify_all();
            // Lock released when guard drops.
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    // R10: print exactly this line.
    println!("DONE waiters=0");
}
