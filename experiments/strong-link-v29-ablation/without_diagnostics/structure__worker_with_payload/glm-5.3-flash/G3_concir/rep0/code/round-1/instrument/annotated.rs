mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// CIR function: main::compute
// Sequential helper performing only local computation.
fn compute() -> i32 {
    let mut t: i32 = 0;
    t = t + 1;
    t = t * 2;
    t
}

// CIR function: main::w1
// The shared variable `acc` lives inside the mutex `m` (protection edge acc -> m).
fn w1(m: Arc<Mutex<i32>>) {
    // mutex_lock {resource: main::m}
    let mut guard = m.lock().unwrap();
    // call {func: main::compute}
    let t = compute();
    let _ = t;
    // read_shared {resource: main::acc}
    let acc = *guard;
    // write_shared {resource: main::acc, expr: acc + 1}
    *guard = acc + 1;
    // mutex_unlock {resource: main::m}
    drop(guard);
    // return {}
}

// CIR function: main::w2
fn w2(m: Arc<Mutex<i32>>) {
    // mutex_lock {resource: main::m}
    let mut guard = m.lock().unwrap();
    // call {func: main::compute}
    let t = compute();
    let _ = t;
    // read_shared {resource: main::acc}
    let acc = *guard;
    // write_shared {resource: main::acc, expr: acc + 1}
    *guard = acc + 1;
    // mutex_unlock {resource: main::m}
    drop(guard);
    // return {}
}

// CIR function: main::main
fn main() { crate::cir_trace::init();
    // Resource m (Mutex) guarding shared var acc (init 0).
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("m_mutex0#1292", 0i32));

    // scope {funcs: [main::w1, main::w2]}
    let m_for_w1 = Arc::clone(&m);
    let m_for_w2 = Arc::clone(&m);

    let handle_w1 = crate::cir_trace::spawn("w1#1439", move || w1(m_for_w1));
    let handle_w2 = crate::cir_trace::spawn("w2#1496", move || w2(m_for_w2));

    // Join every spawned thread.
    handle_w1.join().unwrap();
    handle_w2.join().unwrap();

    // Terminal line required by the requirements document.
    println!("DONE done=1");
 crate::cir_trace::finish();}
