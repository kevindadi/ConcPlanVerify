mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    cv: Condvar,
    m1: Mutex<i32>,
    m2: Mutex<i32>,
    announced: Mutex<i32>,
    done: Mutex<i32>,
}

fn waiter1(shared: Arc<Shared>) {
    // s1: mutex_lock m1
    let mut a_guard = shared.m1.lock().unwrap();
    // s2-s4: read announced, a2 = a+1, write announced
    {
        let mut ann = shared.announced.lock().unwrap();
        let a = *ann;
        let a2 = a + 1;
        *ann = a2;
    }
    // s5: branch announced == 2
    loop {
        let ann = shared.announced.lock().unwrap();
        if *ann == 2 {
            break;
        }
        drop(ann);
        // s6: condvar_wait cv on m1
        a_guard = shared.cv.wait(a_guard).unwrap();
    }
    // s8: mutex_unlock m1
    drop(a_guard);
    // s9: mutex_lock m2
    let mut d_guard = shared.m2.lock().unwrap();
    // s10-s12: read done, d2 = d+1, write done
    {
        let mut d = shared.done.lock().unwrap();
        let dv = *d;
        let d2 = dv + 1;
        *d = d2;
    }
    // s13: mutex_unlock m2
    drop(d_guard);
    // s14: return
}

fn waiter2(shared: Arc<Shared>) {
    // s1: mutex_lock m2
    let m2_guard = shared.m2.lock().unwrap();
    // s2: mutex_lock m1
    let mut a_guard = shared.m1.lock().unwrap();
    // s3-s5: read announced, a2 = a+1, write announced
    {
        let mut ann = shared.announced.lock().unwrap();
        let a = *ann;
        let a2 = a + 1;
        *ann = a2;
    }
    // s6: branch announced == 2
    loop {
        let ann = shared.announced.lock().unwrap();
        if *ann == 2 {
            break;
        }
        drop(ann);
        // s7: condvar_wait cv on m1
        a_guard = shared.cv.wait(a_guard).unwrap();
    }
    // s9: mutex_unlock m1
    drop(a_guard);
    // s10: mutex_unlock m2
    drop(m2_guard);
    // s11: mutex_lock m2
    let mut d_guard = shared.m2.lock().unwrap();
    // s12-s14: read done, d2 = d+1, write done
    {
        let mut d = shared.done.lock().unwrap();
        let dv = *d;
        let d2 = dv + 1;
        *d = d2;
    }
    // s15: mutex_unlock m2
    drop(d_guard);
    // s16: return
}

fn notifier(shared: Arc<Shared>) {
    // s1: mutex_lock m1
    let m1_guard = shared.m1.lock().unwrap();
    // s2: read announced
    loop {
        let ann = shared.announced.lock().unwrap();
        // s3: branch a == 2
        if *ann == 2 {
            break;
        }
        drop(ann);
        // s4: goto s2
    }
    // s5: mutex_unlock m1
    drop(m1_guard);
    // s6: mutex_lock m2
    let m2_guard = shared.m2.lock().unwrap();
    // s7: condvar_notify_all cv
    shared.cv.notify_all();
    // s8: mutex_unlock m2
    drop(m2_guard);
    // s9: return
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        cv: Condvar::new_named("shared_condvar0"),
        m1: Mutex::new_named("shared_mutex0", 0),
        m2: Mutex::new_named("shared_mutex1", 0),
        announced: Mutex::new_named("shared_mutex2", 0),
        done: Mutex::new_named("shared_mutex3", 0),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let t1 = cir_trace::spawn("t1", move || waiter1(s1));
    let t2 = cir_trace::spawn("t2", move || waiter2(s2));
    let t3 = cir_trace::spawn("t3", move || notifier(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
