mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // Channel with capacity 1: a slot guarded by a mutex, plus two semaphores
    // to track emptiness/fullness.
    let slot: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new_named("res_mutex0", None));
    let empty = Semaphore::new_named("empty_semaphore0", 1); // initially empty
    let full = Semaphore::new_named("full_semaphore0", 0);  // initially not full

    // Shared lock m.
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let slot_s = Arc::clone(&slot);
    let empty_s = Arc::clone(&empty);
    let full_s = Arc::clone(&full);
    let m_s = Arc::clone(&m);

    let sender = cir_trace::spawn("sender", move || {
        for v in [1, 2] {
            // Acquire shared lock m, but do not hold it while waiting on channel.
            {
                let _g = m_s.lock().unwrap();
            }

            // Wait until channel is empty, then put value.
            let permit = empty_s.acquire();
            {
                let mut s = slot_s.lock().unwrap();
                *s = Some(v);
            }
            permit.release();
            full_s.acquire().release();
        }
    });

    let slot_r = Arc::clone(&slot);
    let empty_r = Arc::clone(&empty);
    let full_r = Arc::clone(&full);
    let m_r = Arc::clone(&m);

    let receiver = cir_trace::spawn("receiver", move || {
        let mut count = 0;
        for _ in 0..2 {
            // Wait until channel is full, then take value.
            let permit = full_r.acquire();
            let v = {
                let mut s = slot_r.lock().unwrap();
                s.take().unwrap()
            };
            permit.release();
            empty_r.acquire().release();

            // Acquire shared lock m, but not while waiting on channel.
            {
                let _g = m_r.lock().unwrap();
            }

            if v == 2 {
                count = 1;
            }
        }
        println!("DONE done={}", count);
    });

    sender.join().unwrap();
    receiver.join().unwrap();
 cir_trace::finish();}
