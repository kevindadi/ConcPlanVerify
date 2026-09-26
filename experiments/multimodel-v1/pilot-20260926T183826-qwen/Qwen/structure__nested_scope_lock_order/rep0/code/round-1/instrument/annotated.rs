mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let outer_handle = cir_trace::spawn("outer_handle", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || {
            let x1_handle = cir_trace::spawn("x1_handle", {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    // x1: lock a, lock b, unlock b, unlock a
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    drop(_guard_b);
                    drop(_guard_a);
                }
            });

            let x2_handle = cir_trace::spawn("x2_handle", {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    // x2: lock a, lock b, unlock b, unlock a
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    drop(_guard_b);
                    drop(_guard_a);
                }
            });

            x1_handle.join().unwrap();
            x2_handle.join().unwrap();
        }
    });

    outer_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
