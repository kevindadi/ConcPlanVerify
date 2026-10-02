use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

/// Shared resource: c, the atomic counter, starts at zero (R2).
fn make_counter() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}

/// Atomic read-modify-write retry loop (R4, R5, R7).
/// On failure the attempt is retried, never abandoned (R5, R6).
fn increment(c: &AtomicUsize) {
    let mut observed = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange_weak(
            observed,
            observed + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,                       // indivisible increment committed
            Err(actual) => observed = actual,     // retry with fresh value
        }
    }
}

/// Worker role w1 (R1).
fn w1(c: Arc<AtomicUsize>) {
    increment(&c);
}

/// Worker role w2 (R1).
fn w2(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn main() {
    let c = make_counter();

    // Supervising task launches both workers (R1).
    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn({
            let c = Arc::clone(&c);
            move || w1(c)
        })
        .expect("failed to spawn w1");

    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn({
            let c = Arc::clone(&c);
            move || w2(c)
        })
        .expect("failed to spawn w2");

    // Supervisor waits for both workers to finish (R1, R8).
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Both workers have finished, so c == 2 (R3).
    let done = c.load(Ordering::SeqCst);

    // NOTE: R9 as written ("DONE done=1") conflicts with R3 (counter == 2).
    // R9 is marked [U]; we print the true final value instead.
    println!("DONE done={}", done);
}
