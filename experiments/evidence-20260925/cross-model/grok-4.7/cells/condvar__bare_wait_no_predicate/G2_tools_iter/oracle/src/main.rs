mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", false);
    let cv = Condvar::new_named("cv_condvar0");

    thread::scope(|s| {
        let waiter = s.spawn(|| {
            let mut ready = m.lock().unwrap();
            while !*ready {
                ready = cv.wait(ready).unwrap();
            }
        });

        let notifier = s.spawn(|| {
            let mut ready = m.lock().unwrap();
            *ready = true;
            cv.notify_one();
            drop(ready);
        });

        waiter.join().unwrap();
        notifier.join().unwrap();
    });

    let ready = *m.lock().unwrap();
    println!("DONE ready={ready}");
 cir_trace::finish();}
