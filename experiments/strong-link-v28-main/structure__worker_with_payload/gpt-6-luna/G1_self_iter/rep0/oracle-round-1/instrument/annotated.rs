mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Counter {
    acc: usize,
}

fn compute() {
    let result = (0..1_000).fold(0usize, |sum, n| sum.wrapping_add(n));
    std::hint::black_box(result);
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc += 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_observed("m_mutex0#499", Counter { acc: 0 }, __cir_obs_Counter));

    let h1 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w1#581", move || w1(m))
    };
    let h2 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w2#672", move || w2(m))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    assert_eq!(m.lock().unwrap().acc, 2);
    println!("DONE done=1");
 cir_trace::finish();}

fn __cir_obs_Counter(v: &Counter, r: &str) { cir_trace::record_value(&format!("{}::acc", r), v.acc as i64); }
