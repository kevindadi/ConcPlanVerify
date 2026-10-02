mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn a<F>(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    release_sa: F,
)
where
    F: FnOnce(),
{
    let guard = a.lock().unwrap();
    release_sa();
    drop(guard);

    let _permit = sb.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    *guard_a = true;
    drop(guard_b);
    drop(guard_a);
}

fn b<F>(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    release_sb: F,
)
where
    F: FnOnce(),
{
    let guard = a.lock().unwrap();
    release_sb();
    drop(guard);

    let _permit = sa.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
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
    let a_lock = Arc::new(Mutex::new_named("a_lock_mutex0#1002", false));
    let b_lock = Arc::new(Mutex::new_named("b_lock_mutex0#1048", ()));
    let sa = Semaphore::new_named("sa_semaphore0#1082", 1);
    let sb = Semaphore::new_named("sb_semaphore0#1114", 1);

    let permit_sa = sa.acquire();
    let permit_sb = sb.acquire();

    let ha = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("a#1357", move || {
            a(
                a,
                b,
                sa,
                sb,
                move || permit_sa.release(),
            )
        })
    };

    let hb = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        crate::cir_trace::spawn("b#1717", move || {
            b(
                a,
                b,
                sa,
                sb,
                move || permit_sb.release(),
            )
        })
    };

    let _hby = crate::cir_trace::spawn("_hby#1927", bystander);

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
 crate::cir_trace::finish();}
