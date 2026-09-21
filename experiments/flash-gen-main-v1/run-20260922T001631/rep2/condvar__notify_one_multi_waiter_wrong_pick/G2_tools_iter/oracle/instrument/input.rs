use std::sync::{Arc, Condvar, Mutex};

fn main() {
    let pair = Arc::new((Mutex::new((0usize, false)), Condvar::new()));
    let ready = Arc::new((Mutex::new(0usize), Condvar::new()));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let pair = Arc::clone(&pair);
        let ready = Arc::clone(&ready);
        handles.push(std::thread::spawn(move || {
            // Signal readiness.
            {
                let (lock, cvar) = &*ready;
                let mut count = lock.lock().unwrap();
                *count += 1;
                if *count == 2 {
                    cvar.notify_all();
                }
            }

            // Wait until notifier permits.
            let (lock, cvar) = &*pair;
            let mut guard = lock.lock().unwrap();
            while !guard.1 {
                guard = cvar.wait(guard).unwrap();
            }
        }));
    }

    // Notifier: wait until both waiters are ready.
    {
        let (lock, cvar) = &*ready;
        let mut count = lock.lock().unwrap();
        while *count < 2 {
            count = cvar.wait(count).unwrap();
        }
    }

    // Take the lock, wake all waiters, release.
    {
        let (lock, cvar) = &*pair;
        let mut guard = lock.lock().unwrap();
        guard.1 = true;
        cvar.notify_all();
    }

    for h in handles {
        h.join().unwrap();
    }

    // Count remaining waiters (should be 0).
    let remaining = {
        let (lock, _) = &*pair;
        let guard = lock.lock().unwrap();
        if guard.1 { 0 } else { 2 }
    };

    println!("DONE waiters={}", remaining);
}
