mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Counter {
    c: i32,
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = &mut guard.c;
    *c = *c + 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = &mut guard.c;
    *c = *c + 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#387", Counter { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1#454", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = cir_trace::spawn("w2#528", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
