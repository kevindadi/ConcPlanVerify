use std::sync::atomic::{AtomicUsize, Ordering};

// R2: single shared atomic counter, starts at zero.
static c: AtomicUsize = AtomicUsize::new(0);

// Worker role (used for both w1 and w2).
// R4/R5/R7: atomic read-modify-write retry loop (compare-and-swap).
// A failed or spurious-failed attempt is retried, never abandoned, so
// each worker's increment eventually takes effect as one indivisible step.
fn worker() {
    let mut current = c.load(Ordering::SeqCst);
    loop {
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,               // increment applied atomically
            Err(actual) => {
                // R5: retry with the freshest observed value.
                current = actual;
            }
        }
    }
}

fn w1() {
    worker();
}

fn w2() {
    worker();
}

fn main() {
    // R1: supervising task launches both workers and waits for them.
    let h1 = std::thread::spawn(w1);
    let h2 = std::thread::spawn(w2);

    // R8: joining always terminates; the CAS loop above also terminates
    // because at most one competing increment can ever intervene, so every
    // retry eventually observes a value for which the swap succeeds.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // R3/R6: after both workers finish, the counter is exactly two.
    let done = c.load(Ordering::SeqCst);
    println!("DONE done={}", done);
}
