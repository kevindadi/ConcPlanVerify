use std::sync::atomic::{AtomicUsize, Ordering};

// R2: shared atomic counter c, starts at zero
static C: AtomicUsize = AtomicUsize::new(0);

// Worker role: w1 / w2 (same body, one thread each)
fn worker() {
    // R4/R5: atomic read-modify-write retry loop.
    // A failed compare_exchange is retried, never abandoned,
    // so this worker's increment eventually takes effect (R3, R6).
    loop {
        let cur = C.load(Ordering::Acquire);
        match C.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,          // R7: increment applied as one indivisible step
            Err(_) => continue,      // R5: retry on competing update
        }
    }
}

fn main() {
    // R1: supervising task launches two worker threads (w1, w2)
    // and waits for both to finish.
    let w1 = std::thread::spawn(worker);
    let w2 = std::thread::spawn(worker);

    // R8: both loops terminate under any interleaving; join waits for both.
    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // R9: print exactly this line, then exit.
    println!("DONE done=1");
}
