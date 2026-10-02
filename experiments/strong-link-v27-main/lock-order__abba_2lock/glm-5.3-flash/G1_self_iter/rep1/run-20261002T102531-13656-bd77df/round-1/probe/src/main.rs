// Design notes / defect review:
//
// R1: main thread + two spawned workers named t1, t2.
// R2: a and b are Mutexes; a held Mutex is owned by exactly one worker.
// R3: each worker holds both guards simultaneously during its critical work.
// R4: Mutex::lock() blocks until the lock is free, then continues.
// R5: DEFECT AVOIDED (deadlock): both workers acquire locks in the SAME
//     order (a, then b). Circular wait is impossible, so no schedule can
//     leave each worker holding one lock and waiting on the other.
// R6: main joins both workers before finishing.
// R7: the MutexGuards drop at the end of each closure, releasing both
//     locks before the worker thread terminates.
// R8: with a fixed global lock order and blocking acquisition, every
//     interleaving terminates.
// R9: only main prints, exactly `DONE t1=1 t2=1`.

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a and b.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Worker t1: acquires a then b (global lock order).
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = thread::spawn(move || {
        let _guard_a = a1.lock().unwrap(); // wait until a is free
        let _guard_b = b1.lock().unwrap(); // wait until b is free
        // Critical work: both locks held simultaneously here.
        // Guards drop here (R7): b then a released before t1 finishes.
    });

    // Worker t2: acquires a then b (same global lock order -> no deadlock).
    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    let t2 = thread::spawn(move || {
        let _guard_a = a2.lock().unwrap(); // wait until a is free
        let _guard_b = b2.lock().unwrap(); // wait until b is free
        // Critical work: both locks held simultaneously here.
        // Guards drop here (R7): b then a released before t2 finishes.
    });

    // R6: main waits for both workers to finish.
    t1.join().expect("worker t1 panicked");
    t2.join().expect("worker t2 panicked");

    // R9: exactly one line of output.
    println!("DONE t1=1 t2=1");
}
