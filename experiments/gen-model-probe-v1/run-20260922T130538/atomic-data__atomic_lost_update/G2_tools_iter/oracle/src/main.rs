mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // R2: a single atomic counter shared by both workers, starting at zero.
    let counter = Arc::new(AtomicUsize::new(0));

    // R1: the supervising task (main) launches two worker threads.
    let mut handles = Vec::new();
    for _ in 0..2 {
        let shared = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            // R4/R7: atomic read-modify-write via a compare-and-swap retry
            // loop, so each increment is one indivisible step and a
            // competing update is never silently lost.
            let mut current = shared.load(Ordering::SeqCst);
            loop {
                // R3: each worker adds exactly one.
                match shared.compare_exchange(
                    current,
                    current + 1,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    // R5: success means this worker's increment took effect.
                    Ok(_) => break,
                    // R5: a failed attempt is retried with the observed value,
                    // never abandoned.
                    Err(actual) => current = actual,
                }
            }
        }));
    }

    // R1: wait for both workers to finish. With only two single increments,
    // each CAS loop succeeds within a bounded number of retries, so every
    // interleaving terminates (R8) and the counter can always reach two (R6).
    for handle in handles {
        handle.join().unwrap();
    }

    // R9: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
