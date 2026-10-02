mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#409", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#447", ()));

    let w1_a = Arc::clone(&a);
    let w1_b = Arc::clone(&b);
    let handle1 = crate::cir_trace::spawn("w1#538", move || w1(w1_a, w1_b));

    let w2_a = Arc::clone(&a);
    let w2_b = Arc::clone(&b);
    let handle2 = crate::cir_trace::spawn("w2#658", move || w2(w2_a, w2_b));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
