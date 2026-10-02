mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
    drop(guard);
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let c = Arc::new(Mutex::new_named("c_mutex0#355", 0));

    let c1 = Arc::clone(&c);
    let t1 = crate::cir_trace::spawn("w1#407", move || w1(c1));

    let c2 = Arc::clone(&c);
    let t2 = crate::cir_trace::spawn("w2#481", move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done={}", *c.lock().unwrap());
 crate::cir_trace::finish();}
