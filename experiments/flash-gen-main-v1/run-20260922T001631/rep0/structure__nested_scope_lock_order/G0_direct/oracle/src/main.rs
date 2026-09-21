mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R1: main starts one outer worker.
    let outer = cir_trace::spawn("outer_0", || {
        // R2: outer worker starts a nested group of two inner tasks.
        let a = Arc::new(Mutex::new_named_named("a_mutex0", "outer_mutex0", 0u32));
        let b = Arc::new(Mutex::new_named_named("b_mutex0", "outer_mutex1", 0u32));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            handles.push(cir_trace::spawn("outer_1", move || {
                // R3 & R4: take A then B, holding both at once, same order.
                let mut ga = a.lock().unwrap();
                *ga += 1;
                let mut gb = b.lock().unwrap();
                *gb += 1;
                // both held here
                drop(gb);
                drop(ga);
            }));
        }

        // R5: outer completes only after both inner tasks finished.
        for h in handles {
            h.join().unwrap();
        }
    });

    outer.join().unwrap();

    // R7: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
