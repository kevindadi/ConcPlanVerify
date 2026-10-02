mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c = *c + 1;
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c = *c + 1;
    drop(c);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#279", 0i32));

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#334", move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#408", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
