mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    ready: bool,
}

fn waiter(m: &Arc<Mutex<Shared>>, cv: &Arc<Condvar>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    loop {
        // s2: branch on ready == true
        if guard.ready {
            // s5: mutex_unlock m
            drop(guard);
            // s6: return
            break;
        } else {
            // s3: condvar_wait cv under m; s4: goto s2 (re-check after wake)
            guard = cv.wait(guard).unwrap();
        }
    }
}

fn notifier(m: &Arc<Mutex<Shared>>, cv: &Arc<Condvar>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: write_shared ready = true (while holding the lock)
    guard.ready = true;
    // s3: condvar_notify cv (flag set before the signal)
    cv.notify_one();
    // s4: mutex_unlock m
    drop(guard);
    // s5: return
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#937", Shared { ready: false }, __cir_obs_Shared));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#999"));

    let m_w = Arc::clone(&m);
    let cv_w = Arc::clone(&cv);
    let w = crate::cir_trace::spawn("waiter#1082", move || waiter(&m_w, &cv_w));

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let n = crate::cir_trace::spawn("notifier#1201", move || notifier(&m_n, &cv_n));

    w.join().expect("waiter thread panicked");
    n.join().expect("notifier thread panicked");

    // Read shared state only after joins to print the terminal line.
    let ready = m.lock().unwrap().ready;
    println!("DONE ready={}", ready);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::ready", r), v.ready as i64); }
