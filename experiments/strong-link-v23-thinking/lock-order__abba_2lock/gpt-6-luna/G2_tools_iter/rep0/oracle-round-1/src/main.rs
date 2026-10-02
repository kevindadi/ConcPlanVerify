mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#367", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#405", ()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#510", move || t1(a, b))
    };

    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#644", move || t2(a, b))
    };

    let t1_result = t1_handle.join().unwrap();
    let t2_result = t2_handle.join().unwrap();

    println!("DONE t1={t1_result} t2={t2_result}");
 cir_trace::finish();}
