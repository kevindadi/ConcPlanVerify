mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    flag: i32,
}

fn a(
    a: Arc<Mutex<Shared>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    sa_permit: concir_sync::Permit,
) {
    let a_guard = a.lock().unwrap();
    sa_permit.release();
    drop(a_guard);

    let _sb_permit = sb.acquire();

    let mut a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    a_guard.flag += 1;
    drop(_b_guard);
    drop(a_guard);
}

fn b(
    a: Arc<Mutex<Shared>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    sb_permit: concir_sync::Permit,
) {
    let _sa_permit = sa.acquire();
    let a_guard = a.lock().unwrap();
    sb_permit.release();
    drop(a_guard);

    let mut a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    a_guard.flag += 1;
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
    let a = Arc::new(Mutex::new_observed("a_mutex0#1095", Shared { flag: 0 }, __cir_obs_Shared));
    let b = Arc::new(Mutex::new_named("b_mutex0#1149", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1183", 1);
    let sb = Semaphore::new_named("sb_semaphore0#1215", 1);

    let sa_permit = sa.try_acquire().unwrap();
    let sb_permit = sb.try_acquire().unwrap();

    let ha = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("a#1474", move || a(a, b, sa, sb, sa_permit))
    };

    let hb = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("b#1687", move || b(a, b, sa, sb, sb_permit))
    };

    let _hby = crate::cir_trace::spawn("bystander#1760", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    let guard = a.lock().unwrap();
    println!("DONE a={} b=1", guard.flag);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::flag", r), v.flag as i64); }
