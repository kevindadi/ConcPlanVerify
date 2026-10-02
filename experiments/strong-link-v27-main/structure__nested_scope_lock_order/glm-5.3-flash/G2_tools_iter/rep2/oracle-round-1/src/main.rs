mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: a (lock), b (lock)
    let a = Arc::new(Mutex::new_named("a_mutex0#131", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#169", ()));

    // R1: main task starts one outer worker [U]
    let outer_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("new#326", move || {
            // R2: outer starts a nested group of two inner tasks [U]
            let mut inner_handles = Vec::new();

            for name in ["x1", "x2"] {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                inner_handles.push(cir_trace::spawn("yield_now#623", move || {
                    // R4: both inner tasks take the mutexes in the same
                    // order (a, then b), so no wait cycle can form.
                    let _ga = a.lock().unwrap();
                    // Small yield point to allow interleaving; ordering
                    // of acquisition is still a-then-b for both tasks.
                    thread::yield_now();
                    let _gb = b.lock().unwrap();

                    // R3: at this point the inner task holds both a and b.
                    let _ = name;
                    thread::yield_now();

                    // Locks released here when guards drop.
                }));
            }

            // R5: outer completes only after both inner tasks finish.
            for h in inner_handles {
                h.join().unwrap();
            }
        })
    };

    // R5/R6: wait for outer; every interleaving terminates because
    // lock acquisition order is consistent (a before b) for x1 and x2.
    outer_handle.join().unwrap();

    // R7: print exactly the required line, then exit [U]
    println!("DONE done=1");
 cir_trace::finish();}
