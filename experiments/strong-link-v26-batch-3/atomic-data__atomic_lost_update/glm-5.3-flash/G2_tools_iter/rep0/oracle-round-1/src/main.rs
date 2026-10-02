mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Shared resource: c — atomic counter, starts at zero (R2).
static C: AtomicUsize = AtomicUsize::new(0);

// Worker role: w1 / w2 (R1).
// Each worker adds exactly one to the shared counter (R3) using an
// atomic read-modify-write retry loop (R4). A failed CAS attempt is
// retried, never abandoned (R5), so from every reachable state the
// counter can still reach 2 (R6). The compare-and-swap makes each
// increment a single indivisible step (R7), and since every retry
// loop terminates once the CAS succeeds, every interleaving
// terminates (R8).
fn worker() {
    loop {
        let current = C.load(Ordering::SeqCst);
        // Attempt an indivisible increment; on contention the update
        // fails and we immediately retry with the fresh value.
        match C.compare_exchange_weak(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,   // increment took effect
            Err(_) => continue, // lost the race — retry (R5)
        }
    }
}

fn main() { cir_trace::init();
    // Supervising task launches both workers and waits for both (R1).
    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(worker)
        .expect("failed to spawn w1");
    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(worker)
        .expect("failed to spawn w2");

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Both workers have finished; the counter now equals two (R3).
    let done = C.load(Ordering::SeqCst);
    println!("DONE done={}", done);
 cir_trace::finish();}
