mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
    drop(c);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#271", 0));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = cir_trace::spawn("w1#353", move || w1(m1));
    let t2 = cir_trace::spawn("w2#397", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
