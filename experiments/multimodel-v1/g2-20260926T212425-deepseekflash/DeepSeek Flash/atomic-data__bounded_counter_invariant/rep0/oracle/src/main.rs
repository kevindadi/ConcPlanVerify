mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0i32));
    let sem = Arc::new(Semaphore::new_named("sem_semaphore0", 2));

    let m1 = Arc::clone(&m);
    let s1 = Arc::clone(&sem);
    let w1 = cir_trace::spawn("w1", move || {
        let permit = s1.acquire();
        {
            let mut guard = m1.lock().unwrap();
            *guard += 1;
        }
        permit.release();
    });

    let m2 = Arc::clone(&m);
    let s2 = Arc::clone(&sem);
    let w2 = cir_trace::spawn("w2", move || {
        let permit = s2.acquire();
        {
            let mut guard = m2.lock().unwrap();
            *guard += 1;
        }
        permit.release();
    });

    // Supervising task: wait for both workers to finish.
    w1.join().unwrap();
    w2.join().unwrap();

    let value = {
        let mut guard = done.lock().unwrap();
        *guard = 1;
        *guard
    };

    println!("DONE done={}", value);
 cir_trace::finish();}
