use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Critical section: acquires both locks in a fixed global order (a, then b),
/// does the critical work while holding both, then releases both.
fn worker(name: &'static str, a: &Mutex<()>, b: &Mutex<()>, flag: &AtomicUsize) {
    // Fixed acquisition order for ALL workers => no circular wait => no deadlock (R5, R8).
    let _ga = a.lock().unwrap(); // blocks until `a` is free, then proceeds (R4)
    let _gb = b.lock().unwrap(); // blocks until `b` is free, then proceeds (R4)

    // Critical work while holding BOTH locks (R3).
    let _ = name;

    drop(_gb); // release b (R7)
    drop(_ga); // release a (R7)

    flag.store(1, Ordering::SeqCst);
}

fn main() {
    // Shared resources: two locks, one Arc each so both workers see the same locks (R2).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let t1_flag = Arc::new(AtomicUsize::new(0));
    let t2_flag = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    for (name, flag) in [("t1", t1_flag.clone()), ("t2", t2_flag.clone())] {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || worker(name, &a, &b, &flag)));
    }

    // Main starts both workers and waits for both to finish (R6).
    for h in handles {
        h.join().unwrap();
    }

    // Both workers have finished; flags are final and race-free.
    println!(
        "DONE t1={} t2={}",
        t1_flag.load(Ordering::SeqCst),
        t2_flag.load(Ordering::SeqCst)
    ); // R9
}
