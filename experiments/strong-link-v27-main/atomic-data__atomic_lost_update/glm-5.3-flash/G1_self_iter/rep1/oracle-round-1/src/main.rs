mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Worker role: adds exactly one to the shared counter `c`
// using an atomic read-modify-write retry loop.
fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::Relaxed);
        // Single indivisible read-modify-write step (R7).
        // On failure (lost race or spurious weak-CAS failure),
        // retry rather than abandon (R4, R5).
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,       // increment took effect
            Err(_) => continue,   // retry: update was not lost, just deferred
        }
    }
}

// Supervising task: launches w1 and w2, waits for both (R1).
fn main() { cir_trace::init();
    // Shared atomic counter `c`, starting at zero (R2).
    let c = Arc::new(AtomicUsize::new(0));

    let c_w1 = Arc::clone(&c);
    let c_w2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("worker#1024", move || worker(c_w1)); // w1
    let h2 = cir_trace::spawn("worker#1080", move || worker(c_w2)); // w2

    // Wait for both workers; join gives happens-before,
    // so the final load sees both increments.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Both workers finished: counter is exactly 2 (R3).
    let done = c.load(Ordering::Relaxed);
    println!("DONE done={}", done);
 cir_trace::finish();}
