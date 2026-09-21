mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let pair = Arc::new((Mutex::new_named("pair_mutex0", false), Condvar::new_named("pair_condvar0")));

    let waiter_pair = Arc::clone(&pair);
    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cvar) = &*waiter_pair;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    });

    let notifier_pair = Arc::clone(&pair);
    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cvar) = &*notifier_pair;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _cvar) = &*pair;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
