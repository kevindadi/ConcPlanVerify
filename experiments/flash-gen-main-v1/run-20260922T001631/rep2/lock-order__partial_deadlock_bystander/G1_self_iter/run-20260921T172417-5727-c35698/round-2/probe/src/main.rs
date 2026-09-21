use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

struct Shared {
    lock_a: Mutex<()>,
    lock_b: Mutex<()>,
    permit1: (Mutex<bool>, Condvar),
    permit2: (Mutex<bool>, Condvar),
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

    let bystander_shared = Arc::clone(&shared);
    let bystander = thread::spawn(move || {
        loop {
            let _ = bystander_shared.a.lock().unwrap();
            thread::sleep(Duration::from_millis(1));
        }
    });

    // Worker 1: signals permit1, waits for permit2, then acquires locks in
    // the global order lock_a -> lock_b.
    let w1_shared = Arc::clone(&shared);
    let worker1 = thread::spawn(move || {
        {
            let mut p1 = w1_shared.permit1.0.lock().unwrap();
            *p1 = true;
            w1_shared.permit1.1.notify_all();
        }
        {
            let mut p2 = w1_shared.permit2.0.lock().unwrap();
            while !*p2 {
                p2 = w1_shared.permit2.1.wait(p2).unwrap();
            }
        }
        // Acquire both locks in the global order.
        let _guard_a = w1_shared.lock_a.lock().unwrap();
        let _guard_b = w1_shared.lock_b.lock().unwrap();
        {
            let mut a = w1_shared.a.lock().unwrap();
            *a += 1;
        }
        drop(_guard_b);
        drop(_guard_a);
    });

    // Worker 2: signals permit2, waits for permit1, then acquires locks in
    // the same global order lock_a -> lock_b.
    let w2_shared = Arc::clone(&shared);
    let worker2 = thread::spawn(move || {
        {
            let mut p2 = w2_shared.permit2.0.lock().unwrap();
            *p2 = true;
            w2_shared.permit2.1.notify_all();
        }
        {
            let mut p1 = w2_shared.permit1.0.lock().unwrap();
            while !*p1 {
                p1 = w2_shared.permit1.1.wait(p1).unwrap();
            }
        }
        let _guard_a = w2_shared.lock_a.lock().unwrap();
        let _guard_b = w2_shared.lock_b.lock().unwrap();
        {
            let mut b = w2_shared.b.lock().unwrap();
            *b += 1;
        }
        drop(_guard_b);
        drop(_guard_a);
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    std::mem::forget(bystander);

    let a = *shared.a.lock().unwrap();
    let b = *shared.b.lock().unwrap();
    println!("DONE a={} b={}", a, b);
}
