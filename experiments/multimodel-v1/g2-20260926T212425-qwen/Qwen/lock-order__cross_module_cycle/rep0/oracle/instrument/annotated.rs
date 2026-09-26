mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // Shared resources a and b are protected by semaphores with 1 permit each.
    let sem_a = Semaphore::new_named("sem_a_semaphore0", 1);
    let sem_b = Semaphore::new_named("sem_b_semaphore0", 1);

    // To prevent deadlock (R5), we enforce a global lock ordering: always acquire 'a' before 'b'.
    
    let sem_a_t1 = Arc::clone(&sem_a);
    let sem_b_t1 = Arc::clone(&sem_b);
    let t1_handle = cir_trace::spawn("t1_handle", move || {
        // Acquire resource a first
        let _permit_a = sem_a_t1.acquire();
        // Then acquire resource b
        let _permit_b = sem_b_t1.acquire();
        
        // Both resources held here (R4)
        // Perform work...
        
        // Permits dropped at end of scope (R6)
    });

    let sem_a_t2 = Arc::clone(&sem_a);
    let sem_b_t2 = Arc::clone(&sem_b);
    let t2_handle = cir_trace::spawn("t2_handle", move || {
        // Acquire resource a first
        let _permit_a = sem_a_t2.acquire();
        // Then acquire resource b
        let _permit_b = sem_b_t2.acquire();
        
        // Both resources held here (R4)
        // Perform work...
        
        // Permits dropped at end of scope (R6)
    });

    // Wait for both tasks to finish (R7)
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
