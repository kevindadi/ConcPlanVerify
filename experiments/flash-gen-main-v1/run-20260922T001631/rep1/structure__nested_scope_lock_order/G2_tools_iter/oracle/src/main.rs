mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let outer = cir_trace::spawn("outer_0", move || {
        let inner1 = cir_trace::spawnawn("inner1", "outer_1", move || {
            let mut ga = a1.lock().unwrap();
            let mut gb = b1.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        let inner2 = cir_trace::spawnawn("inner2", "outer_2", move || {
            let mut ga = a2.lock().unwrap();
            let mut gb = b2.lock().unwrap();
            *ga += 1;
            *gb += 1;
        });

        inner1.join().unwrap();
        inner2.join().unwrap();
    });

    outer.join().unwrap();

    let done = *a.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
