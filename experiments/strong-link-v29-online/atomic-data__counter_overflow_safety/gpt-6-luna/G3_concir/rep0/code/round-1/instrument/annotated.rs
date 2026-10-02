mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c += 1;
    }
    drop(c);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#323", 0));

    let w1_m = Arc::clone(&m);
    let w1_handle = crate::cir_trace::spawn("w1#384", move || w1(w1_m));

    let w2_m = Arc::clone(&m);
    let w2_handle = crate::cir_trace::spawn("w2#469", move || w2(w2_m));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
