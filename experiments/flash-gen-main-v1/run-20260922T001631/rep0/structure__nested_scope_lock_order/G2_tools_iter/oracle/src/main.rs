mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // R1: main task starts one outer worker.
    let outer = cir_trace::spawn("outer_0", || {
        // R2: outer worker starts a nested group of two inner tasks.
        let a = Arc::new(Mutex::new_named_named("a_mutex0", "outer_mutex0", 0));
        let b = Arc::new(Mutex::new_named_named("b_mutex0", "outer_mutex1", 0));

        let a1 = Arc::clone(&a);
        let b1 = Arc::clone(&b);
        let inner1 = cir_trace::spawnawn("inner1", "outer_1", move || {
            // R3/R4: take A then B, holding both at once.
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
        });

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let inner2 = cir_trace::spawnawn("inner2", "outer_2", move || {
            // R3/R4: same order A then B.
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        });

        // R5: outer worker completes only after both inner tasks finish.
        inner1.join().unwrap();
        inner2.join().unwrap();
    });

    outer.join().unwrap();

    // R7: print exactly the required line.
    println!("DONE done=1");
 cir_trace::finish();}
