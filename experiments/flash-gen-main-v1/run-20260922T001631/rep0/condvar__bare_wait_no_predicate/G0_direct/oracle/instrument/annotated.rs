mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    flag: bool,
}

fn main() { cir_trace::init();
    let shared = Arc::new((Mutex::new_named("shared_mutex0", Shared { flag: false }), Condvar::new_named("shared_condvar0")));

    let waiter_shared = Arc::clone(&shared);
    let notifier_shared = Arc::clone(&shared);

    let waiter = cir_trace::spawn("waiter", move || {
        let (lock, cvar) = &*waiter_shared;
        let mut guard = lock.lock().unwrap();
        while !guard.flag {
            guard = cvar.wait(guard).unwrap();
        }
        // flag is true here
    });

    let notifier = cir_trace::spawn("notifier", move || {
        let (lock, cvar) = &*notifier_shared;
        let mut guard = lock.lock().unwrap();
        guard.flag = true;
        cvar.notify_one();
        // lock released when guard drops
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    let (lock, _) = &*shared;
    let guard = lock.lock().unwrap();
    println!("DONE ready={}", guard.flag);
 cir_trace::finish();}
