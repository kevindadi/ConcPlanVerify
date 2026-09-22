mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn worker(c: &AtomicUsize) {
    // Atomic read-modify-write retry loop: a failed compare_exchange
    // reports the current value, which becomes the basis for the next
    // attempt, so no competing update is ever silently lost and every
    // increment eventually takes effect.
    let mut old = c.load(Ordering::Acquire);
    loop {
        match c.compare_exchange_weak(old, old + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(actual) => old = actual,
        }
    }
}

fn main() { cir_trace::init();
    // Shared atomic counter, initially zero.
    let c = Arc::new(AtomicUsize::new(0));

    // Supervising task: launch w1 and w2, then wait for both to finish.
    let c1 = Arc::clone(&c);
    let w1 = thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || worker(&c1))
        .expect("failed to spawn w1");

    let c2 = Arc::clone(&c);
    let w2 = thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || worker(&c2))
        .expect("failed to spawn w2");

    w1.join().expect("w1 panicked");
    w2.join().expect("w2 panicked");

    // Both workers have finished; each added exactly one, so c == 2.
    debug_assert_eq!(c.load(Ordering::Acquire), 2);

    println!("DONE done=1");
 cir_trace::finish();}
