mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources declared by module main: mutex a, mutex b.
    let a = Arc::new(Mutex::new_named("a_mutex0#154", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#192", ()));

    // main::main -- spawn t1 and t2, then join both.
    let a_t1 = Arc::clone(&a);
    let b_t1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("h1#332", move || {
        // main::t1
        let g1 = a_t1.lock().unwrap();
        let g2 = b_t1.lock().unwrap();
        drop(g2);
        drop(g1);
        1
    });

    let a_t2 = Arc::clone(&a);
    let b_t2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("h2#584", move || {
        // main::t2
        let g1 = a_t2.lock().unwrap();
        let g2 = b_t2.lock().unwrap();
        drop(g2);
        drop(g1);
        1
    });

    let t1 = h1.join().unwrap();
    let t2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", t1, t2);
 cir_trace::finish();}
