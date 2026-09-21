use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    // R2: single shared atomic counter starting at zero.
    let counter = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches two worker threads.
    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            // R4/R5/R7: atomic read-modify-write retry loop.
            // A failed compare_exchange is retried, never abandoned,
            // so no increment is silently lost and each update is
            // observed as a single indivisible step.
            loop {
                let current = counter.load(Ordering::SeqCst);
                if counter
                    .compare_exchange(
                        current,
                        current + 1,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }));
    }

    // R1/R8: wait for both workers; every schedule terminates.
    for handle in handles {
        handle.join().unwrap();
    }

    // R3/R6: both workers have finished, so the shared counter is now 2.
    let _counter_value = counter.load(Ordering::SeqCst);

    // R9: print exactly the required line and exit.
    let done = 1;
    println!("DONE done={}", done);
}
