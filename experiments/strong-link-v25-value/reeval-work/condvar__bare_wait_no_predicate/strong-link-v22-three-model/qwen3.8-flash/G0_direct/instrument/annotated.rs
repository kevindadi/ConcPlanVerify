mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let ready = Arc::new((Mutex::new_named("ready_mutex0#101", false), Condvar::new_named("ready_condvar0#122")));

    let ready_clone1 = Arc::clone(&ready);
    let notifier_handle = cir_trace::spawn("notifier_handle#201", move || {
        let (lock, cvar) = &*ready_clone1;
        let mut flag = lock.lock().unwrap();
        *flag = true;
        cvar.notify_all();
        // Lock is released when `flag` goes out of scope
    });

    let ready_clone2 = Arc::clone(&ready);
    let waiter_handle = cir_trace::spawn("waiter_handle#496", move || {
        let (lock, cvar) = &*ready_clone2;
        let mut flag = lock.lock().unwrap();
        while !*flag {
            flag = cvar.wait(flag).unwrap();
        }
        // Lock is released when `flag` goes out of scope
    });

    notifier_handle.join().unwrap();
    waiter_handle.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
