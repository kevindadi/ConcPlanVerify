mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let counter = Arc::new(Mutex::new_named("counter_mutex0", 0u8));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            let mut guard = counter.lock().unwrap();
            *guard += 1;
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
