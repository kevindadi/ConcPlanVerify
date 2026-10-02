use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// Worker role: performs exactly one increment of the shared counter `c`
// using an atomic read-modify-write retry loop (compare-and-swap).
// A failed CAS is retried, never abandoned, so the increment always
// eventually takes effect and is applied as a single indivisible step.
fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,      // increment succeeded atomically
            Err(_) => continue,  // competing update: retry, never give up
        }
    }
}

// Supervising task: launches w1 and w2, waits for both to finish,
// then prints the required output line and exits.
fn main() {
    // Shared resource `c`: single atomic counter starting at zero.
    let c = Arc::new(AtomicUsize::new(0));

    let c_for_w1 = Arc::clone(&c);
    let c_for_w2 = Arc::clone(&c);

    // Launch worker w1.
    let h1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || worker(c_for_w1))
        .expect("failed to spawn w1");

    // Launch worker w2.
    let h2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || worker(c_for_w2))
        .expect("failed to spawn w2");

    // Wait for both workers to finish. Every interleaving terminates:
    // each CAS retry loop only spins while another update intervenes,
    // and each worker performs a finite amount of work once its CAS
    // succeeds, so no schedule can deadlock or livelock forever.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // At this point both increments have taken effect (c == 2).
    // Print exactly the required line, then exit.
    println!("DONE done=1");
}
