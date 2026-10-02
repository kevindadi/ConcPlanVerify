mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    *guard = *guard + 1;
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    *guard = *guard + 1;
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(Mutex::new_named("c_mutex0#277", 0));

    let c1 = Arc::clone(&c);
    let handle1 = crate::cir_trace::spawn("w1#334", move || w1(c1));

    let c2 = Arc::clone(&c);
    let handle2 = crate::cir_trace::spawn("w2#413", move || w2(c2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
