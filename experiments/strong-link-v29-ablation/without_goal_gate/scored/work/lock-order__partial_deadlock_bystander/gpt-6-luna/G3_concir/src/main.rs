mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::{Permit, Semaphore};
use std::sync::{Arc};
use std::thread;

fn a(
    lock_a: Arc<Mutex<i32>>,
    lock_b: Arc<Mutex<()>>,
    sa_seed: Permit<'static>,
    sb: Arc<Semaphore>,
) {
    {
        let _guard = lock_a.lock().unwrap();
    }

    sa_seed.release();

    let _sb_permit = sb.acquire();
    let mut flag = lock_a.lock().unwrap();
    let guard_b = lock_b.lock().unwrap();
    *flag = 1;
    drop(guard_b);
    drop(flag);
}

fn b(
    lock_a: Arc<Mutex<i32>>,
    lock_b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb_seed: Permit<'static>,
) {
    let _sa_permit = sa.acquire();
    let mut flag = lock_a.lock().unwrap();

    sb_seed.release();

    let guard_b = lock_b.lock().unwrap();
    *flag = 1;
    drop(guard_b);
    drop(flag);
}

fn bystander() {
    loop {
        if 0 < 1 {
            continue;
        } else {
            return;
        }
    }
}

fn main() { crate::cir_trace::init();
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0#950", 0));
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0#992", ()));

    let sa: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("sa_semaphore0#1071", 1)));
    let sb: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("sb_semaphore0#1149", 1)));
    let sa_seed = sa.acquire();
    let sb_seed = sb.acquire();

    let ha = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sb = Arc::clone(sb);
        crate::cir_trace::spawn("a#1364", move || a(lock_a, lock_b, sa_seed, sb))
    };

    let hb = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(sa);
        crate::cir_trace::spawn("b#1566", move || b(lock_a, lock_b, sa, sb_seed))
    };

    let _hby = crate::cir_trace::spawn("bystander#1643", move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    let flag = *lock_a.lock().unwrap();
    println!("DONE a={} b={}", flag, flag);
 crate::cir_trace::finish();}
