mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Deadlock-free three-worker / three-lock design.
//
// Workers form a cycle of lock needs (t1: a+b, t2: b+c, t3: c+a), which
// would deadlock if each acquired its locks in the listed order. To satisfy
// R5/R8/R9, all workers acquire locks in one global order (a before b before
// c). This breaks the circular wait while every worker still holds its two
// required locks simultaneously during its critical section (R3, R4).
fn main() { cir_trace::init();
    // Shared resources: the three locks.
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    let mut workers = Vec::new();

    // t1: needs a and b. Acquires a, then b (respects global order).
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        workers.push(
            thread::Builder::new()
                .name("t1".to_string())
                .spawn(move || {
                    let guard_a = a.lock().unwrap();
                    let guard_b = b.lock().unwrap();
                    // Critical work performed while holding both a and b.
                    critical_work(1);
                    // Locks released here as the guards go out of scope (R6).
                    drop(guard_b);
                    drop(guard_a);
                })
                .unwrap(),
        );
    }

    // t2: needs b and c. Acquires b, then c (respects global order).
    {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        workers.push(
            thread::Builder::new()
                .name("t2".to_string())
                .spawn(move || {
                    let guard_b = b.lock().unwrap();
                    let guard_c = c.lock().unwrap();
                    // Critical work performed while holding both b and c.
                    critical_work(2);
                    drop(guard_c);
                    drop(guard_b);
                })
                .unwrap(),
        );
    }

    // t3: needs c and a. Acquires a first, then c, so that all workers
    // follow the same global order (a < b < c); this is what prevents the
    // circular wait and guarantees every schedule terminates.
    {
        let c = Arc::clone(&c);
        let a = Arc::clone(&a);
        workers.push(
            thread::Builder::new()
                .name("t3".to_string())
                .spawn(move || {
                    let guard_a = a.lock().unwrap();
                    let guard_c = c.lock().unwrap();
                    // Critical work performed while holding both c and a.
                    critical_work(3);
                    drop(guard_c);
                    drop(guard_a);
                })
                .unwrap(),
        );
    }

    // Main thread waits for all three workers to finish (R7).
    for worker in workers {
        worker.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}

// A small deterministic stand-in for the critical work each worker performs
// while holding its pair of locks.
fn critical_work(id: u64) {
    let mut acc = id;
    for i in 0..1024u64 {
        acc = acc.wrapping_mul(31).wrapping_add(i);
    }
    let _ = acc;
}
