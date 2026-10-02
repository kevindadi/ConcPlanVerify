mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    acc: i32,
}

fn compute() {
    let mut tmp = 0;
    tmp = 1;
    let _ = tmp;
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    compute();
    guard.acc = 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#437", Shared { acc: 0 }));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = cir_trace::spawn("w1#535", move || w1(m1));
    let h2 = cir_trace::spawn("w2#579", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
