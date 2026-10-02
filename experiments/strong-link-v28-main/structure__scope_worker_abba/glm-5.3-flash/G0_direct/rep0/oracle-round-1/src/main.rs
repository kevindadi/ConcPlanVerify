mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: a and b (R1: two mutexes contended by both workers)
    let a = Arc::new(Mutex::new_named("a_mutex0#164", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#202", ()));

    // R1: main task starts a group of two workers w1 and w2
    // R5: both workers take the mutexes in the same order (a, then b),
    //     so no wait cycle can form.
    // R6: a worker that cannot take a held mutex waits (Mutex::lock blocks).
    // R7: fixed lock ordering guarantees every interleaving terminates.

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = cir_trace::spawn("w1#607", move || {
        // R2: take a, then b; at this point worker holds both at the same time
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // R3: both guards drop here, releasing each mutex once work is finished
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = cir_trace::spawn("w2#948", move || {
        // R2: take a, then b; at this point worker holds both at the same time
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // R3: both guards drop here, releasing each mutex once work is finished
    });

    // R8: the group finishes only after both workers have completed
    w1.join().unwrap();
    w2.join().unwrap();

    // R9: print exactly this line, then exit
    println!("DONE done=1");
 cir_trace::finish();}
