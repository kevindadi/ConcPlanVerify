// R1: one main thread plus three worker threads (t1, t2, t3).
// R2: three shared locks a, b, c; each is a Mutex held by at most one worker.
// R3: t1 needs a and b; t2 needs b and c; t3 needs c and a.
// R4: each worker holds both of its locks while doing its critical work.
// R5/R8/R9: deadlock is impossible because every worker acquires its locks
//     in a single global order (a < b < c). t1 takes a then b; t2 takes b
//     then c; t3 (which needs c and a) takes a then c. A circular wait can
//     never form, so every schedule terminates.
// R6: each worker releases both locks (via explicit drops) before finishing.
// R7: main starts all three workers and joins them before finishing.
// R10: the program prints exactly `DONE done=1` and exits.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a, b, c (R2).
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // Worker t1: needs a and b; acquires in global order a < b.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap(); // acquire a first
        let _gb = b1.lock().unwrap(); // then b
        // Critical work while holding both locks (R4).
        drop(_gb); // release b before finishing (R6)
        drop(_ga); // release a before finishing (R6)
    });

    // Worker t2: needs b and c; acquires in global order b < c.
    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = thread::spawn(move || {
        let _gb = b2.lock().unwrap(); // acquire b first
        let _gc = c2.lock().unwrap(); // then c
        // Critical work while holding both locks (R4).
        drop(_gc); // release c before finishing (R6)
        drop(_gb); // release b before finishing (R6)
    });

    // Worker t3: needs c and a; acquires in global order a < c
    // (a is taken first even though the worker "needs" c and a,
    // preserving the global lock order that prevents deadlock).
    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = thread::spawn(move || {
        let _ga = a3.lock().unwrap(); // acquire a first
        let _gc = c3.lock().unwrap(); // then c
        // Critical work while holding both locks (R4).
        drop(_gc); // release c before finishing (R6)
        drop(_ga); // release a before finishing (R6)
    });

    // Main starts all three workers and finishes only after all
    // three have finished (R1, R7). Joining guarantees every worker
    // has terminated, whatever order they ran in (R8, R9).
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    // R10: print exactly this line, then exit.
    println!("DONE done=1");
}
