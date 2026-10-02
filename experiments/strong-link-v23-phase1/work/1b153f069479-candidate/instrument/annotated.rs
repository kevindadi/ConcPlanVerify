mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn main() { cir_trace::init();
    let c = AtomicUsize::new(0);
    let c_ref = &c;

    let w1_handle = cir_trace::spawn("w1_handle#152", move || {
        // Each worker adds exactly one to the shared counter using an atomic read-modify-write retry loop.
        loop {
            let current = c_ref.load(Ordering::SeqCst);
            let next = current + 1;
            if c_ref.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
            // Retry on failure
        }
    });

    let w2_handle = cir_trace::spawn("w2_handle#598", move || {
        // Each worker adds exactly one to the shared counter using an atomic read-modify-write retry loop.
        loop {
            let current = c_ref.load(Ordering::SeqCst);
            let next = current + 1;
            if c_ref.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
            // Retry on failure
        }
    });

    // Wait for both workers to finish
    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    // After both workers have finished, the counter should be 2.
    // The requirement says to print "DONE done=1" - this seems odd given the counter should be 2.
    // Let me re-read: R9 says print exactly `DONE done=1`. 
    // Perhaps "done=1" refers to something else? Or maybe it's a typo and should reflect the counter value?
    // But the spec says EXACTLY `DONE done=1`, so we must print that.
    
    println!("DONE done=1");
 cir_trace::finish();}
