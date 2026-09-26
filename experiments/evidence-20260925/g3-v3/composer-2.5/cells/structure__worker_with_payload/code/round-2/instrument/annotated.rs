mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::Arc;
use std::thread;

use cir_trace::sync::Mutex;

struct State {
    acc: i32,
}

fn compute() -> i32 {
    let payload = 1;
    let result = payload;
    result
}

fn w1(m: Arc<Mutex<State>>) {
    let mut guard = m.lock();
    let _ = compute();
    guard.acc += 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<State>>) {
    let mut guard = m.lock();
    let _ = compute();
    guard.acc += 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", State { acc: 0 }));

    let m_w1 = Arc::clone(&m);
    let m_w2 = Arc::clone(&m);

    thread::scope(|s| {
        s.spawn(move || w1(m_w1));
        s.spawn(move || w2(m_w2));
    });

    println!("DONE done=1");
 cir_trace::finish();}
