mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // ch1: rendezvous channel (capacity 0) implemented with two semaphores.
    let ch1_ready = Semaphore::new_named("ch1_ready_semaphore0", 0);   // receiver -> sender: "I'm waiting"
    let ch1_data = Semaphore::new_named("ch1_data_semaphore0", 0);    // sender -> receiver: "data is here"

    // ch2: another rendezvous channel used to confirm completion.
    let ch2_ready = Semaphore::new_named("ch2_ready_semaphore0", 0);
    let ch2_data = Semaphore::new_named("ch2_data_semaphore0", 0);

    // Shared lock occasionally used by both roles.
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0u32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: wait until receiver is ready, then hand over value.
        ch1_ready.acquire();
        let value = 1u32;
        ch1_data.release();

        // Rendezvous on ch2 to confirm receiver got it.
        ch2_ready.acquire();
        ch2_data.release();

        // Use the lock again after channel operations.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        value
    });

    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Signal readiness on ch1, then wait for the value.
        ch1_ready.release();
        ch1_data.acquire();
        let received = 1u32;

        // Confirm on ch2.
        ch2_ready.release();
        ch2_data.acquire();

        // Use the lock again after channel operations.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        received
    });

    let sv = s.join().unwrap();
    let rv = r.join().unwrap();

    println!("DONE done={}", sv + rv - 1);
 cir_trace::finish();}
