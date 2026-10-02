mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    let current = *guard;
    *guard = current + 1;
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    let current = *guard;
    *guard = current + 1;
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(Mutex::new_named("c_mutex0#331", 0));

    let w1_handle = {
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("w1#403", move || w1(c))
    };
    let w2_handle = {
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("w2#501", move || w2(c))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
