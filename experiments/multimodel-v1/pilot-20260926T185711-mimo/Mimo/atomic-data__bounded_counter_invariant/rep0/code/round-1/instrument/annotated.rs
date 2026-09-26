mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Create the shared counter protected by mutex m
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    
    // Spawn two worker threads
    let mut handles = Vec::new();
    
    for _ in 0..2 {
        let m = Arc::clone(&m);
        handles.push(thread::spawn(move || {
            // Worker: lock mutex, increment counter, unlock mutex
            let mut guard = m.lock().unwrap();
            *guard += 1;
            // Mutex automatically unlocks when guard goes out of scope
        }));
    }
    
    // Wait for both workers to finish
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Print the required terminal line
    println!("DONE done=1");
 cir_trace::finish();}
