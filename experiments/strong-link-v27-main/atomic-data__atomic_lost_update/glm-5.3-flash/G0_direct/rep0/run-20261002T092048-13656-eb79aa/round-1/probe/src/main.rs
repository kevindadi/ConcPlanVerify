// Supervising task launches two worker threads (w1, w2) that share a single
// atomic counter (c). Each worker increments c exactly once using an atomic
// read-modify-write retry loop (compare-and-swap), so no update can ever be
// silently lost and every increment appears indivisible. The supervisor waits
// for both workers to finish, then prints the required line and exits.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// Shared resource: c — the atomic counter, starting at zero.
static c: AtomicUsize = AtomicUsize::new(0);

// Worker role: performs exactly one increment of c via a CAS retry loop.
// On a failed (contended) update attempt, the attempt is retried, never
// abandoned, so the increment always eventually takes effect.
fn worker() {
    loop {
        // Read the current value.
        let current = c.load(Ordering::Relaxed);
        // Atomic read-modify-write: if c still holds `current`, replace it
        // with `current + 1` in one indivisible step. No partial update is
        // ever observable, and a competing update is never lost — we simply
        // retry with the freshly observed value.
        match c.compare_exchange(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,   // increment succeeded
            Err(_) => continue, // lost the race: retry, never abandon
        }
    }
}

fn main() {
    // Supervising task: launch w1 and w2, then wait for both to finish.
    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(worker)
        .expect("failed to spawn w1");

    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(worker)
        .expect("failed to spawn w2");

    // Waiting for both workers guarantees termination under every possible
    // schedule/interleaving: the CAS loop always makes forward progress
    // because a failed attempt immediately retries against the latest value.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // At this point both workers have finished; the counter has reached two.
    // Print exactly the required line, then exit.
    println!("DONE done=1");
}
