mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    flag: bool,
    lock: Mutex<()>,
    cond: Condvar,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        flag: false,
        lock: Mutex::new_named("shared_mutex0", ()),
        cond: Condvar::new_named("shared_condvar0"),
    });

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = cir_trace::spawn("waiter", move || {
        let mut guard = waiter_shared.lock.lock().unwrap();
        while !waiter_shared.flag {
            guard = waiter_shared.cond.wait(guard).unwrap();
        }
    });

    let notifier = cir_trace::spawn("notifier", move || {
        let _guard = notifier_shared.lock.lock().unwrap();
        notifier_shared.flag = true;
        notifier_shared.cond.notify_one();
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let ready = {
        let _guard = shared.lock.lock().unwrap();
        shared.flag
    };

    println!("DONE ready={}", ready);
 cir_trace::finish();}
