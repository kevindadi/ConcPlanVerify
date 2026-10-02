mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

/// Shared counter `c`, starts at zero (R2).
/// Each worker performs an atomic read-modify-write retry loop (R4–R7).
fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::Relaxed);
        // Single indivisible RMW on success; a failed attempt (including a
        // spurious failure of the weak variant) is retried, never abandoned.
        match c.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,      // increment took effect
            Err(_) => continue,  // retry: competing update won this round
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    // Supervisor launches w1 and w2 (R1).
    let mut handles = Vec::new();
    for _ in 0..2 {
        let c = Arc::clone(&c);
        handles.push(thread::spawn(move || worker(c)));
    }

    // Wait for both workers to finish (R1, R8).
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    // After both workers finish, the counter is necessarily 2 (R3, R6).
    let done = c.load(Ordering::SeqCst);
    // NOTE: R9 as written ("DONE done=1") contradicts R3/R6; the consistent
    // final value is 2, so that is what is printed.
    println!("DONE done={}", done);
 cir_trace::finish();}
