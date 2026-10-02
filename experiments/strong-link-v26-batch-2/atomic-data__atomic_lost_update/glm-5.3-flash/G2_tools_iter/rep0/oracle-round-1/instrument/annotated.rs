mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Shared resource: c, the atomic counter, starting at zero (R2).
// Workers: w1 and w2 (R1).

fn worker(name: &'static str, c: Arc<AtomicUsize>) {
    // Atomic read-modify-write retry loop (R4, R5):
    // a failed update attempt is retried, never abandoned, so no
    // increment is ever silently lost (R6).
    //
    // compare_exchange_weak performs the read-modify-write as a single
    // indivisible step (R7): no partial update is observable, and a
    // competing update causes the CAS to fail and the loop to retry.
    loop {
        let current = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // Increment took effect atomically.
                break;
            }
            Err(_) => {
                // Update failed due to contention: retry (R5, R6).
                // Every retry path still allows the counter to reach 2.
                continue;
            }
        }
    }
    let _ = name; // w1 / w2 identified by role
}

fn main() { cir_trace::init();
    // Supervising task: launches w1 and w2 and waits for both (R1).
    let c = Arc::new(AtomicUsize::new(0)); // starts at zero (R2)

    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("worker#1409", move || worker("w1", c1));

    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("worker#1493", move || worker("w2", c2));

    // Wait for both workers to finish (R1). Every interleaving
    // terminates: the CAS loop only retries on contention and each
    // retry has a chance to succeed, so progress is guaranteed (R8).
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Both workers have added exactly one (R3), so c == 2.
    let done = c.load(Ordering::Acquire);
    println!("DONE done={}", done);
 cir_trace::finish();}
