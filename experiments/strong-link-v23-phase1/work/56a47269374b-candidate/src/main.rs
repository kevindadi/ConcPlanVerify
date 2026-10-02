mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    // Shared atomic counter starting at zero
    let c = Arc::new(AtomicUsize::new(0));

    // Clone the Arc for each worker thread
    let c_w1 = Arc::clone(&c);
    let c_w2 = Arc::clone(&c);

    // Launch two worker threads w1 and w2
    let handle_w1 = cir_trace::spawn("handle_w1#358", move || {
        // Worker w1: increment the shared counter using an atomic read-modify-write retry loop
        loop {
            let current = c_w1.load(Ordering::SeqCst);
            let next = current + 1;
            // Attempt to update with compare-and-swap; if it fails due to contention, retry
            match c_w1.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    let handle_w2 = cir_trace::spawn("handle_w2#896", move || {
        // Worker w2: increment the shared counter using an atomic read-modify-write retry loop
        loop {
            let current = c_w2.load(Ordering::SeqCst);
            let next = current + 1;
            // Attempt to update with compare-and-swap; if it fails due to contention, retry
            match c_w2.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    // Wait for both workers to finish
    handle_w1.join().unwrap();
    handle_w2.join().unwrap();

    // Verify that the counter equals two
    let final_value = c.load(Ordering::SeqCst);
    assert_eq!(final_value, 2, "Counter should be 2 after both increments");

    // Print exactly the line `DONE done=1` and then exit
    println!("DONE done=1");
 cir_trace::finish();}
