mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let counter = Arc::new(Mutex::new_named("counter_mutex0", 0u8));

    let c1 = Arc::clone(&counter);
    let c2 = Arc::clone(&counter);

    let h1 = cir_trace::spawn("h1", move || {
        let mut guard = c1.lock().unwrap();
        *guard += 1;
    });

    let h2 = cir_trace::spawn("h2", move || {
        let mut guard = c2.lock().unwrap();
        *guard += 1;
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
