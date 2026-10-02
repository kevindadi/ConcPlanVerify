mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct State {
    acc: i32,
}

fn compute() {
    let mut tmp = 0;
    tmp = tmp + 1;
}

fn w1(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = guard.acc + 1;
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#447", State { acc: 0 }, __cir_obs_State));

    let m1 = Arc::clone(&m);
    let t1 = crate::cir_trace::spawn("w1#514", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = crate::cir_trace::spawn("w2#588", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = i32::from(m.lock().unwrap().acc == 2);
    println!("DONE done={}", done);
 crate::cir_trace::finish();}

fn __cir_obs_State(v: &State, r: &str) { crate::cir_trace::record_value(&format!("{}::acc", r), v.acc as i64); }
