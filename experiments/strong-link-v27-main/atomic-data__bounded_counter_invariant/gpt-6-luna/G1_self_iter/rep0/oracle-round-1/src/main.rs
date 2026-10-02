mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<u8>>) {
    let mut c = m.lock().unwrap();
    assert!(*c < 2);
    *c += 1;
}

fn w2(m: Arc<Mutex<u8>>) {
    let mut c = m.lock().unwrap();
    assert!(*c < 2);
    *c += 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#285", 0_u8));

    let m1 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1#340", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = cir_trace::spawn("w2#414", move || w2(m2));

    let r1 = t1.join();
    let r2 = t2.join();

    r1.unwrap();
    r2.unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
