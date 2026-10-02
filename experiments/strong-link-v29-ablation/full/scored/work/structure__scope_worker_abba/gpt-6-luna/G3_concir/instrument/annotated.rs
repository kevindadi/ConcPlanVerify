mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#413", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#451", ()));

    let w1_a = Arc::clone(&a);
    let w1_b = Arc::clone(&b);
    let w1_handle = crate::cir_trace::spawn("w1#544", move || w1(w1_a, w1_b));

    let w2_a = Arc::clone(&a);
    let w2_b = Arc::clone(&b);
    let w2_handle = crate::cir_trace::spawn("w2#666", move || w2(w2_a, w2_b));

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
