mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // R2: shared atomic counter starting at zero.
    let counter = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches two worker threads.
    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            // R4/R5/R7: atomic read-modify-write retry loop.
            // A failed attempt is retried, never abandoned, and the
            // compare_exchange makes each increment indivisible.
            loop {
                let current = counter.load(Ordering::SeqCst);
                let next = current + 1;
                if counter
                    .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }
        }));
    }

    // R1: wait for both workers to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    // R3: counter now equals two (internal state).
    // R9: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
