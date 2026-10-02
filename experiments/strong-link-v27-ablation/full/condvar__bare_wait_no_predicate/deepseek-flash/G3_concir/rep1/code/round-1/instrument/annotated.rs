mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#96", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#140"));

    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter = cir_trace::spawn("waiter#238", move || {
        let mut guard = waiter_m.lock().unwrap();
        while *guard == false {
            guard = waiter_cv.wait(guard).unwrap();
        }
        drop(guard);
    });

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier = cir_trace::spawn("notifier#531", move || {
        let mut guard = notifier_m.lock().unwrap();
        *guard = true;
        notifier_cv.notify_one();
        drop(guard);
    });

    waiter.join().unwrap();
    notifier.join().unwrap();

    println!("DONE ready=true");
 cir_trace::finish();}
