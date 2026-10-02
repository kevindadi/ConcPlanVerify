mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn main() { cir_trace::init();
    // Shared atomic counter starting at zero
    let c = AtomicUsize::new(0);

    // Clone the counter for each worker thread (since AtomicUsize is not Clone, we use references)
    let c1 = &c;
    let c2 = &c;

    // Launch two worker threads w1 and w2
    let handle1 = cir_trace::spawn("handle1#354", move || {
        // Worker w1: increment the shared counter using an atomic read-modify-write retry loop
        loop {
            let current = c1.load(Ordering::SeqCst);
            let next = current + 1;
            // Attempt to update with compare-and-swap; if it fails due to contention, retry
            match c1.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    let handle2 = cir_trace::spawn("handle2#886", move || {
        // Worker w2: increment the shared counter using an atomic read-modify-write retry loop
        loop {
            let current = c2.load(Ordering::SeqCst);
            let next = current + 1;
            // Attempt to update with compare-and-swap; if it fails due to contention, retry
            match c2.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    });

    // Wait for both workers to finish
    handle1.join().unwrap();
    handle2.join().unwrap();

    // Verify that the counter equals two
    let final_value = c.load(Ordering::SeqCst);
    assert_eq!(final_value, 2, "Counter should be 2 after both increments");

    // Print exactly the line `DONE done=1` and then exit
    println!("DONE done=1");
 cir_trace::finish();}
