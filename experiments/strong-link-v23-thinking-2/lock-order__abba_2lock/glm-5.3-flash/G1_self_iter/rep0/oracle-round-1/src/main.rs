mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: lock a, lock b.
    let a = Arc::new(Mutex::new_named("a_mutex0#176", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#214", ()));

    // Per-worker completion flags for the final report.
    let t1_done = Arc::new(AtomicUsize::new(0));
    let t2_done = Arc::new(AtomicUsize::new(0));

    // R1: main thread spawns exactly two workers, t1 and t2.
    let handles = vec![
        {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            let done = Arc::clone(&t1_done);
            thread::spawn(move || worker("t1", a, b, done))
        },
        {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            let done = Arc::clone(&t2_done);
            thread::spawn(move || worker("t2", a, b, done))
        },
    ];

    // R6: main thread finishes only after both workers have finished.
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    // R9: exactly one line of output, then exit.
    println!(
        "DONE t1={} t2={}",
        t1_done.load(Ordering::SeqCst),
        t2_done.load(Ordering::SeqCst)
    );
 cir_trace::finish();}

fn worker(
    name: &'static str,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    done: Arc<AtomicUsize>,
) {
    // R2/R3/R5: acquire BOTH locks in the same global order (a, then b).
    // A consistent order makes a deadlock cycle impossible, so no worker
    // can hold one lock while forever waiting on the other.
    let _ga = a.lock().expect("lock a poisoned"); // R4: blocks until free
    let _gb = b.lock().expect("lock b poisoned"); // R4: blocks until free

    // Critical work while holding both locks (R3).
    // (Simulated work; both guards remain alive here.)
    let _ = name;

    // R7: guards drop here, releasing both locks before the worker finishes.
    drop(_gb);
    drop(_ga);

    done.fetch_add(1, Ordering::SeqCst);
}
