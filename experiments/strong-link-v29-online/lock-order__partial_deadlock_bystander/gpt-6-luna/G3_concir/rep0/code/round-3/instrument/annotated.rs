mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn a(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    drop(a.lock().unwrap());

    sa.acquire().release();

    drop(sb.acquire());

    let mut flag = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    *flag = true;
    drop(_b_guard);
    drop(flag);
}

fn b(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    drop(b.lock().unwrap());

    sb.acquire().release();

    drop(sa.acquire());

    let mut flag = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    *flag = true;
    drop(_b_guard);
    drop(flag);
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
    let a_lock = Arc::new(Mutex::new_named("a_lock_mutex0#917", false));
    let b_lock = Arc::new(Mutex::new_named("b_lock_mutex0#963", ()));
    let sa = Semaphore::new_named("sa_semaphore0#997", 1);
    let sb = Semaphore::new_named("sb_semaphore0#1029", 1);

    let a_arg = Arc::clone(&a_lock);
    let b_arg = Arc::clone(&b_lock);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let ha = crate::cir_trace::spawn("a#1193", move || a(a_arg, b_arg, sa_arg, sb_arg));

    let a_arg = Arc::clone(&a_lock);
    let b_arg = Arc::clone(&b_lock);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let hb = crate::cir_trace::spawn("b#1405", move || b(a_arg, b_arg, sa_arg, sb_arg));

    let _hby = crate::cir_trace::spawn("bystander#1477", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
 crate::cir_trace::finish();}
