use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: two locks, `a` and `b`.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Counters so the main thread can report each worker's completion.
    let t1_done = Arc::new(AtomicUsize::new(0));
    let t2_done = Arc::new(AtomicUsize::new(0));

    // Worker t1: acquires `a` then `b` (fixed global order).
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&t1_done);
        let handle1 = thread::spawn(move || {
            // R4: if a lock is busy, lock() blocks until it becomes free.
            let _guard_a = a.lock().unwrap(); // hold lock `a`...
            let _guard_b = b.lock().unwrap(); // ...then acquire lock `b` (R3: both held at once)

            // Critical work while holding both locks.
            done.fetch_add(1, Ordering::SeqCst);

            // R7: guards drop here (before the thread finishes), releasing both locks.
        });
        handle1.join().unwrap();
    }

    // Worker t2: acquires `a` then `b` in the SAME order, so the two workers
    // can never each hold one lock while waiting for the other (R5: no deadlock).
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&t2_done);
        let handle2 = thread::spawn(move || {
            let _guard_a = a.lock().unwrap();
            let _guard_b = b.lock().unwrap();

            done.fetch_add(1, Ordering::SeqCst);

            // R7: both locks released before the thread finishes.
        });
        handle2.join().unwrap();
    }

    // R6: main started both workers and only finishes after both have joined.
    // R9: print exactly the required line, then exit.
    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::SeqCst),
        t2_done.load(Ordering::SeqCst)
    );
}
