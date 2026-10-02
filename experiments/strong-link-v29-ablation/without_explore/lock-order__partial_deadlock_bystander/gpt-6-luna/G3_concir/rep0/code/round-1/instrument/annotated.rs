mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn a(a_lock: Arc<Mutex<bool>>, b_lock: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    let guard = a_lock.lock().unwrap();
    sa.acquire().release();
    drop(guard);

    sb.acquire().release();

    let mut guard_a = a_lock.lock().unwrap();
    let guard_b = b_lock.lock().unwrap();
    *guard_a = true;
    drop(guard_b);
    drop(guard_a);
}

fn b(a_lock: Arc<Mutex<bool>>, b_lock: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    let guard = a_lock.lock().unwrap();
    sb.acquire().release();
    drop(guard);

    sa.acquire().release();

    let mut guard_a = a_lock.lock().unwrap();
    let guard_b = b_lock.lock().unwrap();
    *guard_a = true;
    drop(guard_b);
    drop(guard_a);
}

fn bystander() {
    let mut i: i32 = 0;
    loop {
        i = i.wrapping_add(1);
    }
}

fn main() { crate::cir_trace::init();
    let a_lock = Arc::new(Mutex::new_named("a_lock_mutex0#936", false));
    let b_lock = Arc::new(Mutex::new_named("b_lock_mutex0#982", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1016", 1);
    let sb = Semaphore::new_named("sb_semaphore0#1048", 1);

    let ha = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("a#1232", move || a(a_lock, b_lock, sa, sb))
    };

    let hb = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("b#1464", move || b(a_lock, b_lock, sa, sb))
    };

    let _hby = crate::cir_trace::spawn("_hby#1536", bystander);

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
 crate::cir_trace::finish();}
