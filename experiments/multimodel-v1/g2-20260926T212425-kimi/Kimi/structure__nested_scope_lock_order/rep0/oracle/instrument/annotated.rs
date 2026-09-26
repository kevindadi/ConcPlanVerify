mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0i32));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0i32));

    let outer = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("clone", move || {
            // Nested group: two inner tasks x1 and x2.
            let x1 = {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                cir_trace::spawn("x1", move || {
                    // Lock order: a then b.
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    // Both mutexes are held here.
                })
            };
            let x2 = {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                cir_trace::spawn("x2", move || {
                    // Same lock order: a then b, so no wait cycle can form.
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    // Both mutexes are held here.
                })
            };
            // Outer completes only after both inner tasks have finished.
            x1.join().unwrap();
            x2.join().unwrap();
        })
    };

    outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
