mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct AState {
    flag: bool,
}

fn a(
    a_lock: Arc<Mutex<AState>>,
    b_lock: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    let guard = a_lock.lock().unwrap();
    let permit = sa.acquire();
    permit.release();
    drop(guard);

    let _permit = sb.acquire();

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    a_guard.flag = true;
    a_guard.flag = false;
    drop(_b_guard);
    drop(a_guard);
}

fn b(
    a_lock: Arc<Mutex<AState>>,
    b_lock: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    let guard = b_lock.lock().unwrap();
    let permit = sb.acquire();
    permit.release();
    drop(guard);

    let _permit = sa.acquire();

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    a_guard.flag = true;
    a_guard.flag = false;
    drop(_b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut phase = 0;
    loop {
        if phase == 0 {
            phase = 1;
        } else {
            phase = 0;
        }
    }
}

fn main() { crate::cir_trace::init();
    let a_lock = Arc::new(Mutex::new_observed("a_lock_mutex0#1200", AState { flag: false }, __cir_obs_AState));
    let b_lock = Arc::new(Mutex::new_named("b_lock_mutex0#1263", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1297", 0);
    let sb = Semaphore::new_named("sb_semaphore0#1329", 0);

    let ha = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("a#1513", move || a(a_lock, b_lock, sa, sb))
    };

    let hb = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("b#1745", move || b(a_lock, b_lock, sa, sb))
    };

    let _hby = crate::cir_trace::spawn("bystander#1817", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
 crate::cir_trace::finish();}

fn __cir_obs_AState(v: &AState, r: &str) { crate::cir_trace::record_value(&format!("{}::flag", r), v.flag as i64); }
