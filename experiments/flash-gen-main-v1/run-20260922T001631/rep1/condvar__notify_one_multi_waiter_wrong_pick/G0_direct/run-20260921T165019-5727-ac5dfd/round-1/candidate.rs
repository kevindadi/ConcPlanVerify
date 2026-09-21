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

    let mut handles = Vec::new();

    // Two waiter threads.
    for _ in 0..2 {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();

            // Mark this waiter as ready to wait.
            guard.ready += 1;
            cvar.notify_all();

            // Wait until the notifier has signalled.
            while !guard.notified {
                guard = cvar.wait(guard).unwrap();
            }

            // Count this waiter as released.
            guard.released += 1;
        }));
    }

    // Notifier thread.
    {
        let shared = Arc::clone(&shared);
        handles.push(thread::spawn(move || {
            let (lock, cvar) = &*shared;
            let mut guard = lock.lock().unwrap();

            // Wait until both waiters are ready.
            while guard.ready < 2 {
                guard = cvar.wait(guard).unwrap();
            }

            // Wake every waiter.
            guard.notified = true;
            cvar.notify_all();
        }));
    }

    // Join all threads.
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all waiters completed.
    let (lock, _) = &*shared;
    let guard = lock.lock().unwrap();
    let remaining = 2 - guard.released;

    println!("DONE waiters={}", remaining);
}
