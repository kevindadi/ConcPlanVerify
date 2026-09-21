mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    flag: Mutex<bool>,
    cond: Condvar,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        flag: Mutex::new_named("shared_mutex0", false),
        cond: Condvar::new_named("shared_condvar0"),
    });

    let waiter_shared = Arc::clone(&shared);
    let waiter = cir_trace::spawn("waiter", move || {
        let mut flag = waiter_shared.flag.lock().unwrap();
        while !*flag {
            flag = waiter_shared.cond.wait(flag).unwrap();
        }
    });

    let notifier_shared = Arc::clone(&shared);
    let notifier = cir_trace::spawn("notifier", move || {
        let mut flag = notifier_shared.flag.lock().unwrap();
        *flag = true;
        notifier_shared.cond.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = *shared.flag.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
