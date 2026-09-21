mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R1: main task starts one outer worker.
    let outer = cir_trace::spawn("outer_0", || {
        let a = Arc::new(Mutex::new_named_named("a_mutex0", "outer_mutex0", 0));
        let b = Arc::new(Mutex::new_named_named("b_mutex0", "outer_mutex1", 0));

        // R2: outer worker starts a nested group of two inner tasks.
        let mut handles = Vec::new();
        for _ in 0..2 {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            handles.push(cir_trace::spawn("outer_1", move || {
                // R3 & R4: take A then B, same order in both tasks.
                let mut ga = a.lock().unwrap();
                let mut gb = b.lock().unwrap();
                *ga += 1;
                *gb += 1;
            }));
        }

        // R5: outer worker completes only after both inner tasks finish.
        for h in handles {
            h.join().unwrap();
        }

        // R7: print exactly the required line.
        println!("DONE done=1");
    });

    outer.join().unwrap();
 cir_trace::finish();}
