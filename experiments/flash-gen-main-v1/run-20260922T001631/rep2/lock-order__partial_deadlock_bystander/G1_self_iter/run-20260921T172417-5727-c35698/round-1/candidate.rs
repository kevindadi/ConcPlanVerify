use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// Two locks and two counting permits.
struct Shared {
    // Two locks (mutexes) that workers must hold simultaneously.
    lock_a: Mutex<()>,
    lock_b: Mutex<()>,
    // Two counting permits used as a handshake.
    // permit1: worker1 signals it has taken its first lock.
    // permit2: worker2 signals it has taken its first lock.
    permit1: (Mutex<bool>, Condvar),
    permit2: (Mutex<bool>, Condvar),
    // Counters to verify both workers entered critical sections.
    a: Mutex<i32>,
    b: Mutex<i32>,
}

fn main() {
    let shared = Arc::new(Shared {
        lock_a: Mutex::new(()),
        lock_b: Mutex::new(()),
        permit1: (Mutex::new(false), Condvar::new()),
        permit2: (Mutex::new(false), Condvar::new()),
        a: Mutex::new(0),
        b: Mutex::new(0),
    });

    // Bystander task: keeps making progress, never finishes on its own.
    let bystander_shared = Arc::clone(&shared);
    let bystander = thread::spawn(move || {
        loop {
            // Do some independent work that doesn't hold the workers' locks.
            let _ = bystander_shared.a.lock().unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    });

    // Worker 1
    let w1_shared = Arc::clone(&shared);
    let worker1 = thread::spawn(move || {
        // Take first lock.
        let _guard_a = w1_shared.lock_a.lock().unwrap();

        // Signal permit1: we have taken our first lock.
        {
            let mut p1 = w1_shared.permit1.0.lock().unwrap();
            *p1 = true;
            w1_shared.permit1.1.notify_all();
        }

        // Wait for worker2 to take its first lock (permit2).
        {
            let mut p2 = w1_shared.permit2.0.lock().unwrap();
            while !*p2 {
                p2 = w1_shared.permit2.1.wait(p2).unwrap();
            }
        }

        // Now take second lock.
        let _guard_b = w1_shared.lock_b.lock().unwrap();

        // Critical section: hold both locks.
        {
            let mut a = w1_shared.a.lock().unwrap();
            *a += 1;
        }

        // Release locks (drop guards) before finishing.
        drop(_guard_b);
        drop(_guard_a);
    });

    // Worker 2
    let w2_shared = Arc::clone(&shared);
    let worker2 = thread::spawn(move || {
        // Take first lock.
        let _guard_b = w2_shared.lock_b.lock().unwrap();

        // Signal permit2: we have taken our first lock.
        {
            let mut p2 = w2_shared.permit2.0.lock().unwrap();
            *p2 = true;
            w2_shared.permit2.1.notify_all();
        }

        // Wait for worker1 to take its first lock (permit1).
        {
            let mut p1 = w2_shared.permit1.0.lock().unwrap();
            while !*p1 {
                p1 = w2_shared.permit1.1.wait(p1).unwrap();
            }
        }

        // Now take second lock.
        let _guard_a = w2_shared.lock_a.lock().unwrap();

        // Critical section: hold both locks.
        {
            let mut b = w2_shared.b.lock().unwrap();
            *b += 1;
        }

        // Release locks (drop guards) before finishing.
        drop(_guard_a);
        drop(_guard_b);
    });

    // Main thread waits only for the two workers.
    worker1.join().unwrap();
    worker2.join().unwrap();

    // Bystander keeps running; we don't join it.
    // Detach it by forgetting the handle.
    std::mem::forget(bystander);

    let a = *shared.a.lock().unwrap();
    let b = *shared.b.lock().unwrap();
    println!("DONE a={} b={}", a, b);
}
