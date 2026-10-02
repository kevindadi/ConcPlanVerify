use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// R2: shared resource `c` — a single atomic counter starting at zero.
static C: AtomicUsize = AtomicUsize::new(0);

// R4/R5/R7: each worker performs its increment as an atomic
// read-modify-write retry loop. A failed compare-and-swap is retried,
// never abandoned, so no update can be silently lost and every
// increment takes effect as one indivisible step.
//
// The strong `compare_exchange` is used deliberately: the weak variant
// may fail spuriously, and although a spurious failure is also retried,
// the strong variant gives a hard guarantee that the loop cannot
// livelock, satisfying R8 (every schedule terminates).
fn increment() {
    loop {
        let current = C.load(Ordering::Acquire);
        match C.compare_exchange(
            current,
            current + 1,
            Ordering::AcqRel, // success: RMW with release so the increment is indivisible
            Ordering::Acquire, // failure: re-read with acquire before retrying
        ) {
            Ok(_) => break,     // increment committed in a single step (R7)
            Err(_) => continue, // R5: retry rather than abandon
        }
    }
}

// Roles: w1 and w2 (R1 entity names).
fn w1() {
    increment();
}

fn w2() {
    increment();
}

fn main() {
    // R1: the supervising task (main) launches the two worker threads
    // and waits for both of them to finish.
    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // At this point both workers have completed their increments, so the
    // counter `c` holds 2 (R3, R6: from every reachable state it was
    // always possible to reach two, and it has now done so).
    debug_assert_eq!(C.load(Ordering::SeqCst), 2);

    // R9 [U]: print exactly this line, then exit.
    println!("DONE done=1");
}
