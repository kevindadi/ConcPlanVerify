mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let counter = Arc::new(Mutex::new_named("counter_mutex0", 0i32));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        let handle = cir_trace::spawn("handle", move || {
            let mut guard = counter.lock().unwrap();
            if *guard < 1 {
                *guard += 1;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_value = *counter.lock().unwrap();
    println!("DONE done={}", final_value);
 cir_trace::finish();}
