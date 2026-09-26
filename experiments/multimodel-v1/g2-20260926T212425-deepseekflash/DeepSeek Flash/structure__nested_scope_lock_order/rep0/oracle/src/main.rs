mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let done = Arc::new(Semaphore::new_named("done_semaphore0", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let done1 = Arc::clone(&done);

    let outer = cir_trace::spawn("clone", move || {
        let x1 = cir_trace::spawn("x1", move || {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
            drop(_gb);
            drop(_ga);
            let permit = done1.try_acquire();
            if let Some(p) = permit {
                p.release();
            }
        });

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let done2 = Arc::clone(&done);

        let x2 = cir_trace::spawn("x2", move || {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
            drop(_gb);
            drop(_ga);
            let permit = done2.try_acquire();
            if let Some(p) = permit {
                p.release();
            }
        });

        x1.join().unwrap();
        x2.join().unwrap();
    });

    outer.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
