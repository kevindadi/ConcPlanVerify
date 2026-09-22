mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc};
use std::thread;

// Each worker must hold BOTH locks `a` and `b` at the same time while it
// performs its critical work. Both workers acquire the locks in the same
// global order (first `a`, then `b`). This consistent ordering makes the
// classic circular-wait deadlock impossible: no cycle can ever form where
// each worker holds one lock and waits for the other.
fn worker(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    done: Arc<AtomicU32>,
) {
    // Block until `a` is free, then acquire it (R4).
    let guard_a = a.lock().unwrap();
    // While still holding `a`, block until `b` is free, then acquire it.
    let guard_b = b.lock().unwrap();

    // ---- critical section: both locks are held simultaneously (R3) ----
    done.store(1, Ordering::SeqCst);
    // -------------------------------------------------------------------

    // Release both locks before finishing (R7).
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    // The two shared locks (R2).
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    // Completion flags, written inside the critical section.
    let done_t1 = Arc::new(AtomicU32::new(0));
    let done_t2 = Arc::new(AtomicU32::new(0));

    // Main starts worker t1 (R6).
    let t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&done_t1);
        thread::Builder::new()
            .name("t1".to_string())
            .spawn(move || worker(a, b, done))
            .unwrap()
    };

    // Main starts worker t2 (R6).
    let t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&done_t2);
        thread::Builder::new()
            .name("t2".to_string())
            .spawn(move || worker(a, b, done))
            .unwrap()
    };

    // Main finishes only after both workers have finished (R6).
    t1.join().unwrap();
    t2.join().unwrap();

    // R9: print exactly this line and exit.
    println!(
        "DONE t1={} t2={}",
        done_t1.load(Ordering::SeqCst),
        done_t2.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
