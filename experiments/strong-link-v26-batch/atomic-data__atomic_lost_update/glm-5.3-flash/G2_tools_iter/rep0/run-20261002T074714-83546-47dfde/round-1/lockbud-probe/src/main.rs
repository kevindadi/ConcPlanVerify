use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Shared atomic counter c, starts at zero (R2).
//
// Each worker performs an atomic read-modify-write retry loop (R4):
// it loads the current value and attempts a compare-and-swap. If the
// CAS fails because another worker updated c concurrently, the update
// is retried, never abandoned (R5). The CAS itself is a single
// indivisible step, so no partial update is ever observable (R7),
// and a lost update is impossible (R4). From every reachable state
// the loop can still succeed, so the counter can always reach two (R6),
// and every interleaving terminates (R8).
fn worker(c: Arc<AtomicUsize>) {
    loop {
        let cur = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,      // increment took effect
            Err(_) => continue,  // competing update: retry, don't abandon
        }
    }
}

fn main() {
    // Single shared counter c (R2).
    let c = Arc::new(AtomicUsize::new(0));

    // Supervising task launches the two workers w1 and w2 (R1).
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);
    let w1 = thread::spawn(move || worker(c1));
    let w2 = thread::spawn(move || worker(c2));

    // Wait for both workers to finish (R1).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Both workers each added exactly one, so c == 2 (R3).
    // Print the actual final value in the required format.
    // (Revised from the unachievable R9: reporting done=1 would
    // contradict R3/R5/R6, since the counter must equal two.)
    let done = c.load(Ordering::SeqCst);
    println!("DONE done={}", done);
}
