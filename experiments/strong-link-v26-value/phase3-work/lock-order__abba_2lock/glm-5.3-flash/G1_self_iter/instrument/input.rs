use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// R1: two worker functions, t1 and t2, driven by one shared worker body.
// R5/R8: fixed global lock order (a, then b) makes circular wait impossible,
//        so no interleaving can deadlock.
fn worker(name: &'static str, a: &Mutex<()>, b: &Mutex<()>, done: &AtomicUsize) {
    // R4: lock() blocks until the lock is free, then continues.
    let ga = a.lock().unwrap(); // acquire a first (global order)
    let gb = b.lock().unwrap(); // then b

    // R3: critical work while holding BOTH locks.
    let _critical_work: () = {
        let _ = name; // placeholder for real work under both locks
    };

    // R7: release each lock before the worker finishes.
    drop(gb);
    drop(ga);

    done.fetch_add(1, Ordering::SeqCst);
}

fn main() {
    // Shared resources: a and b (R2: both workers share the same two locks).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let t1_done = Arc::new(AtomicUsize::new(0));
    let t2_done = Arc::new(AtomicUsize::new(0));

    // R1: main thread starts both workers.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let d1 = Arc::clone(&t1_done);
    let h1 = thread::spawn(move || worker("t1", &a1, &b1, &d1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let d2 = Arc::clone(&t2_done);
    let h2 = thread::spawn(move || worker("t2", &a2, &b2, &d2));

    // R6: main finishes only after both workers have finished.
    h1.join().expect("t1 panicked");
    h2.join().expect("t2 panicked");

    // R9: exactly one line, then exit.
    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::SeqCst),
        t2_done.load(Ordering::SeqCst)
    );
}
