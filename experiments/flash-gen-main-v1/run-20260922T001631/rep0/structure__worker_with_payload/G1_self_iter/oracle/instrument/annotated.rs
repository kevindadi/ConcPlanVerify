mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn helper() {
    // sequential helper routine that performs only local computation
    let mut sum = 0u64;
    for i in 0..1000 {
        sum = sum.wrapping_add(i);
    }
    std::hint::black_box(sum);
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
