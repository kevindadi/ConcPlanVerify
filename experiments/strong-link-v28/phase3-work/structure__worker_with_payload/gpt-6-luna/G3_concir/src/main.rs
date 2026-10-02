mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Acc {
    acc: i32,
}

fn compute() -> i32 {
    let mut tmp = 0;
    tmp += 1;
    tmp
}

fn w1(m: Arc<Mutex<Acc>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<Acc>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#429", Acc { acc: 0 }, __cir_obs_Acc));

    let m1 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1#494", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = cir_trace::spawn("w2#568", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_Acc(v: &Acc, r: &str) { cir_trace::record_value(&format!("{}::acc", r), v.acc as i64); }
