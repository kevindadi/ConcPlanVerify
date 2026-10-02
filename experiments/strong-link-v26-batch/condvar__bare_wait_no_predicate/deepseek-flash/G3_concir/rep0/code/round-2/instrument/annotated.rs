mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

/// CIR function `main::waiter` (role: waiter).
///
/// mutex_lock main::m
/// branch: while ready == false -> condvar_wait main::cv / main::m
/// mutex_unlock main::m
fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();

    // branch { cond: "ready == false", then: wait, else: unlock }
    // condvar_wait main::cv is re-entered via the goto back to the branch.
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }

    // mutex_unlock main::m
    drop(guard);
    // return
}

/// CIR function `main::notifier` (role: notifier).
///
/// mutex_lock main::m
/// write_shared main::ready = true
/// condvar_notify main::cv
/// mutex_unlock main::m
fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    // mutex_lock main::m
    let mut guard = m.lock().unwrap();

    // write_shared main::ready = true
    *guard = true;

    // condvar_notify main::cv
    cv.notify_one();

    // mutex_unlock main::m
    drop(guard);
    // return
}

/// CIR function `main::main`.
///
/// scope { funcs: [main::waiter, main::notifier] } then return.
fn main() { cir_trace::init();
    // Shared resources: mutex `m` guards the shared variable `ready` (bool),
    // and `cv` is the paired condition variable.
    let m = Arc::new(Mutex::new_named("m_mutex0#1343", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1387"));

    // scope: start the named roles concurrently, each identified by its CIR name.
    let waiter_m = Arc::clone(&m);
    let waiter_cv = Arc::clone(&cv);
    let waiter_handle = cir_trace::spawn("waiter#1575", move || waiter(waiter_m, waiter_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let notifier_handle = cir_trace::spawn("notifier#1730", move || notifier(notifier_m, notifier_cv));

    // Join every spawned thread.
    waiter_handle.join().unwrap();
    notifier_handle.join().unwrap();

    // After joins, read shared state only for the terminal line.
    let ready = *m.lock().unwrap();
    println!("DONE ready={}", ready);
 cir_trace::finish();}
