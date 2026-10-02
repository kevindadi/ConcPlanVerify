use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// R2: shared atomic counter c, starts at zero.
// R4/R5/R6/R7: each worker does an atomic read-modify-write retry loop;
// a failed attempt is retried, never abandoned, so no update is lost
// and every increment is a single indivisible step.
fn worker(c: &AtomicUsize) {
    loop {
        let current = c.load(Ordering::Relaxed);
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,          // increment took effect atomically
            Err(_) => continue,      // retry: competing update or spurious failure
        }
    }
}

fn main() {
    // R1: supervising task launches two worker threads and waits for both.
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = thread::spawn(move || worker(&c1)); // w1
    let h2 = thread::spawn(move || worker(&c2)); // w2

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // R3: both workers finished, so c == 2.
    // R9 [U] is unsatisfiable as written (it demands done=1, contradicting R3);
    // we print the actual, correct value.
    let done = c.load(Ordering::SeqCst);
    println!("DONE done={}", done);
}
