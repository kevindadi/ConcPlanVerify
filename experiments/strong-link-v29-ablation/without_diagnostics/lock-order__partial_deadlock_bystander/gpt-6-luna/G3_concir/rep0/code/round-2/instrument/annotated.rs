mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn a(
    a_lock: Arc<Mutex<i32>>,
    b_lock: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    // Hold the initial sa permit so it can be released at the CIR signal.
    let sa_permit = sa.acquire();

    let a_guard = a_lock.lock().unwrap();
    sa_permit.release();
    drop(a_guard);

    let _sb_permit = sb.acquire();

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    *a_guard += 1;
    drop(_b_guard);
    drop(a_guard);
}

fn b(
    a_lock: Arc<Mutex<i32>>,
    b_lock: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    // Hold the initial sb permit so it can be released at the CIR signal.
    let sb_permit = sb.acquire();

    let _sa_permit = sa.acquire();
    let a_guard = a_lock.lock().unwrap();
    sb_permit.release();
    drop(a_guard);

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    *a_guard += 1;
    drop(_b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut i = 0;
    loop {
        if i < 0 {
            return;
        }
        i = 1;
        i = 0;
    }
}

fn main() { crate::cir_trace::init();
    let a_lock = Arc::new(Mutex::new_named("a_lock_mutex0#1250", 0));
    let b_lock = Arc::new(Mutex::new_named("b_lock_mutex0#1292", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1326", 1);
    let sb = Semaphore::new_named("sb_semaphore0#1358", 1);

    let ha = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("a#1542", move || a(a_lock, b_lock, sa, sb))
    };

    let hb = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("b#1774", move || b(a_lock, b_lock, sa, sb))
    };

    let _hby = crate::cir_trace::spawn("bystander#1846", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    let a_guard = a_lock.lock().unwrap();
    println!("DONE a={} b=1", *a_guard);
 crate::cir_trace::finish();}
