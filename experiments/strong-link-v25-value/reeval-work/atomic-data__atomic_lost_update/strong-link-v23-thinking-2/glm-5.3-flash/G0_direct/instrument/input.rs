use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// R2: single shared atomic counter, starts at zero.
static c: AtomicUsize = AtomicUsize::new(0);

// R4, R5, R6, R7: atomic read-modify-write retry loop.
// The load and the compare-and-swap together form the read-modify-write;
// a failed (or spurious) CAS attempt is retried, never abandoned, so each
// worker's increment eventually takes effect and can never be silently lost.
// The compare_exchange itself is a single indivisible atomic step, so no
// partial update is ever observable. Since the only operations on `c` are
// increments by a bounded number of workers, the retry loop always
// terminates, no matter how the two workers interleave (R8).
fn increment() {
    loop {
        let cur = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,          // increment took effect atomically
            Err(_) => continue,      // R5: retry, do not abandon
        }
    }
}

// R1, R3: each worker adds exactly one to the shared counter.
fn w1() {
    increment();
}

fn w2() {
    increment();
}

fn main() {
    // R1: supervising task launches the two workers and waits for both.
    let h1 = thread::spawn(w1);
    let h2 = thread::spawn(w2);

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // At this point both workers have finished, so c == 2 (R3).
    // R9: print exactly the required line, then exit.
    println!("DONE done=1");
}
