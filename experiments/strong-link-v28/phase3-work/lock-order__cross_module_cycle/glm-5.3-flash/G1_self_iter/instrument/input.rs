//! Deadlock-free two-resource design.
//!
//! Repaired defect: an earlier draft had t1 acquire (a, then b) while t2
//! acquired (b, then a) — a circular wait, i.e. deadlock (violates R5/R9).
//! Repair: one global lock hierarchy. Every task acquires `a` before `b`,
//! so a cycle in the wait-for graph is impossible under any interleaving.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

mod res_a {
    //! R1: this module owns shared resource `a`.
    use std::sync::{Mutex, OnceLock};

    static A: OnceLock<Mutex<()>> = OnceLock::new();

    pub fn a() -> &'static Mutex<()> {
        A.get_or_init(|| Mutex::new(()))
    }
}

mod res_b {
    //! R1: this module owns shared resource `b`.
    use std::sync::{Mutex, OnceLock};

    static B: OnceLock<Mutex<()>> = OnceLock::new();

    pub fn b() -> &'static Mutex<()> {
        B.get_or_init(|| Mutex::new(()))
    }
}

/// Number of tasks that have fully finished (released all resources).
static FINISHED: AtomicUsize = AtomicUsize::new(0);

mod task1 {
    //! R2: task t1 runs in this module.
    use std::sync::atomic::Ordering;

    use super::{res_a, res_b, FINISHED};

    pub fn t1() {
        // R3: t1 declares its dependency on the *other* module's resource `b`
        // by acquiring it here. Global hierarchy respected: a before b.
        let ga = res_a::a().lock().unwrap(); // own module's resource a
        let gb = res_b::b().lock().unwrap(); // dependency on res_b's b

        // R4: both `a` and `b` are held simultaneously while the work runs.
        // (critical section: work using both resources)

        // R6: release each resource before t1 finishes (RAII drop).
        drop(gb);
        drop(ga);

        FINISHED.fetch_add(1, Ordering::SeqCst);
    }
}

mod task2 {
    //! R2: task t2 runs in this module.
    use std::sync::atomic::Ordering;

    use super::{res_a, res_b, FINISHED};

    pub fn t2() {
        // R3: t2 declares its dependency on res_a's `a`.
        // Same global hierarchy: a before b — this is the deadlock repair.
        let ga = res_a::a().lock().unwrap(); // dependency on res_a's a
        let gb = res_b::b().lock().unwrap(); // own module's resource b

        // R4: both `a` and `b` are held simultaneously while the work runs.

        // R6: release each resource before t2 finishes (RAII drop).
        drop(gb);
        drop(ga);

        FINISHED.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    // R7: the starting thread launches both tasks...
    let h1 = thread::spawn(task1::t1);
    let h2 = thread::spawn(task2::t2);

    // R7/R8: ...and finishes only after both tasks (from different
    // modules) have finished.
    h1.join().expect("t1 panicked");
    h2.join().expect("t2 panicked");

    let done = if FINISHED.load(Ordering::SeqCst) == 2 { 1 } else { 0 };

    // R10: exactly one line, then exit.
    println!("DONE done={}", done);
}
