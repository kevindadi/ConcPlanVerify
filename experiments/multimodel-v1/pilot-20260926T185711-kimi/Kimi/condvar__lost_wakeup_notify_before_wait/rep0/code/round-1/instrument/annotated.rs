mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn waiter(pair: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*pair;
    let mut ready = m.lock().unwrap();
    while !*ready {
        ready = cv.wait(ready).unwrap();
    }
    drop(ready);
}

fn notifier(pair: Arc<(Mutex<bool>, Condvar)>) {
    let (m, cv) = &*pair;
    let mut ready = m.lock().unwrap();
    *ready = true;
    cv.notify_one();
    drop(ready);
}

fn main() { cir_trace::init();
    let pair = Arc::new((Mutex::new_named("pair_mutex0", false), Condvar::new_named("pair_condvar0")));

    let waiter_pair = Arc::clone(&pair);
    let notifier_pair = Arc::clone(&pair);

    let w = cir_trace::spawn("waiter", move || waiter(waiter_pair));
    let n = cir_trace::spawn("notifier", move || notifier(notifier_pair));

    w.join().unwrap();
    n.join().unwrap();

    let (m, _) = &*pair;
    let ready = m.lock().unwrap();
    println!("DONE ready={}", *ready);
 cir_trace::finish();}
