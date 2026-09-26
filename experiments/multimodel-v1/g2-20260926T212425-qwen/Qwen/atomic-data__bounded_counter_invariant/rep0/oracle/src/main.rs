mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::thread;
use std::sync::Arc;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // R2: Shared counter c starts at 0.
    let c = Arc::new(cir_trace::sync::Mutex::new_named("c_mutex0", 0));
    
    // Use Semaphore for mutual exclusion as per crate hint, though Mutex works too.
    // Actually, if I use Semaphore, I still need to protect the data. 
    // Usually Semaphore guards the critical section.
    let sem = Semaphore::new_named("sem_semaphore0", 1);

    // R1: Supervising task launches two workers and waits for them.
    let c1 = Arc::clone(&c);
    let s1 = Arc::clone(&sem);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        // R3 & R4: Hold lock (permit) while reading/writing c.
        let _permit = s1.acquire();
        let mut guard = c1.lock().unwrap();
        *guard += 1;
    });

    let c2 = Arc::clone(&c);
    let s2 = Arc::clone(&sem);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        // R3 & R4: Hold lock (permit) while reading/writing c.
        let _permit = s2.acquire();
        let mut guard = c2.lock().unwrap();
        *guard += 1;
    });

    // R6: Join ensures termination.
    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    // R7: Print exactly "DONE done=1".
    println!("DONE done=1");
 cir_trace::finish();}
