use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Shared resource: c (atomic counter), starts at zero (R2).
static C: AtomicUsize = AtomicUsize::new(0);

// Worker body shared by w1 and w2.
// R4/R5: atomic read-modify-write retry loop; a failed attempt is
// retried, never abandoned, so every increment eventually lands.
// R7: compare_exchange is a single indivisible step — no partial
// update is ever observable.
// R8: the loop always terminates: each successful CAS is permanent
// (values only ever increase by 1), so retries are finite.
fn worker() {
    loop {
        let cur = C.load(Ordering::Acquire);
        let next = cur + 1;
        match C.compare_exchange_weak(
            cur,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,   // increment took effect indivisibly
            Err(_) => continue, // lost the race or spurious failure: retry
        }
    }
}

// R1: main is the supervising task; it launches w1 and w2 and waits
// for both to finish.
fn main() {
    let w1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(worker)
        .expect("failed to spawn w1");

    let w2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(worker)
        .expect("failed to spawn w2");

    // Wait for both workers (R1). Joining both also guarantees every
    // possible interleaving terminates before we proceed (R8).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // R3/R6: both increments have taken effect, so c == 2 here, and
    // from every reachable state the counter could still reach 2
    // (failed CAS attempts are retried, never abandoned).
    debug_assert_eq!(C.load(Ordering::Acquire), 2);

    // R9: print exactly this line, then exit.
    println!("DONE done=1");
}
