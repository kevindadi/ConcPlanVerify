//! Design notes (justification of concurrency correctness):
//!
//! R1: `module1` owns shared resource `a` (a Mutex); `module2` owns shared resource `b`.
//! R2: task `t1` runs in `module1`, task `t2` runs in `module2`; both need both resources.
//! R3: each task's signature explicitly takes the other module's resource as a
//!     dependency parameter (`dep_b` for t1, `dep_a` for t2).
//! R4: each task holds both guards simultaneously while performing its work.
//! R5: deadlock freedom is guaranteed by a single global lock order: every task
//!     acquires `a` before `b`, never the reverse. With a consistent order, a
//!     cycle in the wait-for graph is impossible, so no schedule can leave the
//!     two tasks waiting on each other forever.
//! R6: each task explicitly drops both guards before returning.
//! R7: `main` (the starting thread) spawns both tasks and joins both, so it
//!     finishes only after both tasks have finished.
//! R8/R9: both tasks always terminate: lock acquisition under a consistent
//!     order always eventually succeeds (the holder of a lock only holds it for
//!     a finite, non-blocking critical section), so every interleaving terminates.
//! R10: after both joins, the program prints exactly `DONE done=1` and exits.

mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc};

/// Module 1 owns shared resource `a`.
mod module1 {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub struct Module1 {
        pub a: Arc<Mutex<()>>, // resource `a` is owned by this module
    }

    /// Task t1 (runs in module1).
    /// R3: declares its dependency on resource `b`, owned by module2.
    pub fn t1(own_a: Arc<Mutex<()>>, dep_b: Arc<Mutex<()>>) {
        // Global lock order: `a` first, then `b` (R5).
        let ga = own_a.lock().unwrap();
        let gb = dep_b.lock().unwrap();

        // R4: both resources are held simultaneously here.
        let _work = (*ga, *gb); // perform work while holding both

        // R6: release each resource before finishing.
        drop(gb);
        drop(ga);
    }
}

/// Module 2 owns shared resource `b`.
mod module2 {
    use std::sync::{Arc};use crate::cir_trace::sync::{Mutex};

    pub struct Module2 {
        pub b: Arc<Mutex<()>>, // resource `b` is owned by this module
    }

    /// Task t2 (runs in module2).
    /// R3: declares its dependency on resource `a`, owned by module1.
    pub fn t2(own_b: Arc<Mutex<()>>, dep_a: Arc<Mutex<()>>) {
        // Global lock order: `a` first, then `b` (R5).
        // Note: t2 acquires `a` (the other module's resource) BEFORE its own
        // `b`, so it never holds `b` while waiting for `a` — this is what
        // rules out the classic hold-and-wait deadlock.
        let ga = dep_a.lock().unwrap();
        let gb = own_b.lock().unwrap();

        // R4: both resources are held simultaneously here.
        let _work = (*ga, *gb); // perform work while holding both

        // R6: release each resource before finishing.
        drop(gb);
        drop(ga);
    }
}

/// Completion flag, set to 1 once both tasks have finished (R10).
static DONE: AtomicU8 = AtomicU8::new(0);

fn main() { cir_trace::init();
    let m1 = module1::Module1 { a: Arc::new(Mutex::new_named("a#3180", ())) };
    let m2 = module2::Module2 { b: Arc::new(Mutex::new_named("b#3243", ())) };

    // R7: the starting thread (main) launches both tasks.
    let a1 = Arc::clone(&m1.a);
    let b1 = Arc::clone(&m2.b);
    let h1 = cir_trace::spawn("t1#3392", move || module1::t1(a1, b1));

    let a2 = Arc::clone(&m1.a);
    let b2 = Arc::clone(&m2.b);
    let h2 = cir_trace::spawn("t2#3519", move || module2::t2(b2, a2));

    // R7/R8: main finishes only after both tasks have finished.
    h1.join().expect("t1 panicked");
    h2.join().expect("t2 panicked");

    DONE.store(1, Ordering::SeqCst);

    // R10: print exactly this line, then exit.
    println!("DONE done={}", DONE.load(Ordering::SeqCst));
 cir_trace::finish();}
