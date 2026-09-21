mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn helper() {
    // Sequential helper routine performing only local computation.
    let mut acc: u64 = 0;
    for i in 0..1000 {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);
}

fn main() { cir_trace::init();
    let counter = Arc::new(Mutex::new_named("counter_mutex0", 0u32));
    let mut handles = Vec::new();

    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            let mut guard = counter.lock().unwrap();
            helper();
            *guard += 1;
            drop(guard);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let done = *counter.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
