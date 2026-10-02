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
    let m = Arc::new(Mutex::new_named("m_mutex0#285", 0u8));

    let t1 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w1#352", move || w1(m))
    };
    let t2 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w2#443", move || w2(m))
    };

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
