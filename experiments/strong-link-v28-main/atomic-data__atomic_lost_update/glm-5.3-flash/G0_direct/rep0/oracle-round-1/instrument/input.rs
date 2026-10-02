use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// Shared resource: c (atomic counter), starts at zero.
static ZERO_START: usize = 0;

fn worker(name: &'static str, c: &Arc<AtomicUsize>) {
    // Atomic read-modify-write retry loop (R4, R5):
    // a failed update attempt is retried, never abandoned,
    // so this worker's increment eventually takes effect (R6).
    loop {
        let cur = c.load(Ordering::Acquire);
        // compare_exchange performs the increment as a single
        // indivisible read-modify-write step (R7); on failure
        // (a competing update won the race) we simply retry (R5).
        match c.compare_exchange(
            cur,
            cur + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                // Increment of worker `name` took effect.
                break;
            }
            Err(_) => {
                // Lost the race; retry so the update is never lost.
                continue;
            }
        }
    }
}

fn main() {
    // Single shared atomic counter starting at zero (R2).
    let c = Arc::new(AtomicUsize::new(ZERO_START));

    // Supervising task launches two worker threads, w1 and w2 (R1).
    let c1 = Arc::clone(&c);
    let h1 = std::thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || worker("w1", &c1))
        .expect("failed to spawn w1");

    let c2 = Arc::clone(&c);
    let h2 = std::thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || worker("w2", &c2))
        .expect("failed to spawn w2");

    // Wait for both workers to finish (R1, R8):
    // every interleaving terminates because each retry loop
    // only spins while another increment is in progress,
    // and there are exactly two increments to perform.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // Once both workers have finished, c equals two (R3).
    debug_assert_eq!(c.load(Ordering::SeqCst), 2);

    // Required output (R9).
    println!("DONE done=1");
}
