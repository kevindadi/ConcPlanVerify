mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let flag = Arc::new((Mutex::new_named("flag_mutex0", false), Condvar::new_named("flag_condvar0")));

    let waiter_flag = Arc::clone(&flag);
    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cvar) = &*waiter_flag;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
        // flag is true here
    });

    let notifier_flag = Arc::clone(&flag);
    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cvar) = &*notifier_flag;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*flag;
    let ready = lock.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
