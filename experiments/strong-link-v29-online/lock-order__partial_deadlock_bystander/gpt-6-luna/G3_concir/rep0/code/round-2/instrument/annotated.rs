mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct AState {
    flag: bool,
}

fn a(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    drop(a.lock().unwrap());

    sa.acquire().release();

    let permit = sb.acquire();
    std::mem::forget(permit);

    let mut a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    a_guard.flag = true;
    drop(b_guard);
    drop(a_guard);
}

fn b(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    drop(b.lock().unwrap());

    sb.acquire().release();

    let permit = sa.acquire();
    std::mem::forget(permit);

    let mut a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    a_guard.flag = true;
    drop(b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut i = true;
    loop {
        if i == true {
            i = true;
        } else {
            return;
        }
    }
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_observed("a_mutex0#1009", AState { flag: false }, __cir_obs_AState));
    let b = Arc::new(Mutex::new_named("b_mutex0#1067", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1101", 0);
    let sb = Semaphore::new_named("sb_semaphore0#1133", 0);

    let a_arg = Arc::clone(&a);
    let b_arg = Arc::clone(&b);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let ha = crate::cir_trace::spawn("a#1287", move || crate::a(a_arg, b_arg, sa_arg, sb_arg));

    let a_arg = Arc::clone(&a);
    let b_arg = Arc::clone(&b);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let hb = crate::cir_trace::spawn("b#1496", move || crate::b(a_arg, b_arg, sa_arg, sb_arg));

    let hby = crate::cir_trace::spawn("bystander#1574", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();
    drop(hby);

    println!("DONE a=1 b=1");
 crate::cir_trace::finish();}

fn __cir_obs_AState(v: &AState, r: &str) { crate::cir_trace::record_value(&format!("{}::flag", r), v.flag as i64); }
