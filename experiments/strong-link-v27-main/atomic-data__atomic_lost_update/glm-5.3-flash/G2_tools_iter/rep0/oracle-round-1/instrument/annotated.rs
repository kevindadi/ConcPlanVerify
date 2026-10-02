mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // R2: shared atomic counter `c`, starting at zero.
    let c = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches w1 and w2, then waits for both.
    let c_for_w1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("increment#315", move || increment(&c_for_w1));

    let c_for_w2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("increment#409", move || increment(&c_for_w2));

    // Wait for both workers to finish (R1, R8: every schedule terminates).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // R9: the program must print exactly this line and then exit.
    println!("DONE done=1");
 cir_trace::finish();}

// R3: each worker adds exactly one to `c`.
// R4/R5: the update is a read-modify-write retry loop — a failed
//   compare-and-swap is retried with the freshest value, never abandoned,
//   so a competing update can never be silently lost and every worker's
//   increment eventually takes effect.
// R6: from any observable state (0 or 1), a retry with the current value
//   can still succeed, so the counter can always still reach two.
// R7: compare_exchange makes the increment a single indivisible step;
//   no partial update is ever observable.
// R8: the counter only ever increases (0 -> 1 -> 2), so each retry either
//   succeeds or observes a strictly larger value; the loop terminates
//   under every interleaving of w1 and w2.
fn increment(c: &AtomicUsize) {
    let mut current = c.load(Ordering::Acquire);
    loop {
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,   // success ordering: increment is published
            Ordering::Acquire,  // failure ordering: re-read coherently
        ) {
            Ok(_) => return,                   // increment took effect
            Err(actual) => current = actual,   // R5: retry, never abandon
        }
    }
}
