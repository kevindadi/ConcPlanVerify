mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    done: i32,
}

fn x1(a: &Arc<Mutex<Shared>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn x2(a: &Arc<Mutex<Shared>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn outer(a: Arc<Mutex<Shared>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h_x1 = crate::cir_trace::spawn("x1#509", move || {
        x1(&a1, &b1);
    });
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h_x2 = crate::cir_trace::spawn("x2#636", move || {
        x2(&a2, &b2);
    });
    h_x1.join().unwrap();
    h_x2.join().unwrap();
    let mut guard = a.lock().unwrap();
    guard.done = 1;
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_observed("a_mutex0#861", Shared { done: 0 }, __cir_obs_Shared));
    let b = Arc::new(Mutex::new_named("b_mutex0#915", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);
    let h_outer = crate::cir_trace::spawn("outer#1012", move || {
        outer(a_outer, b_outer);
    });

    h_outer.join().unwrap();

    let guard = a.lock().unwrap();
    let done = guard.done;
    drop(guard);
    println!("DONE done={}", done);
 crate::cir_trace::finish();}

fn __cir_obs_Shared(v: &Shared, r: &str) { crate::cir_trace::record_value(&format!("{}::done", r), v.done as i64); }
