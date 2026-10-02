mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let mut work = 0;
    work += 1;

    drop(b_guard);
    drop(a_guard);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let b_guard = b.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work += 1;

    drop(c_guard);
    drop(b_guard);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work += 1;

    drop(c_guard);
    drop(a_guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#687", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#725", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#763", ()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("t1#868", move || t1(a, b))
    };

    let t2_handle = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t2#1002", move || t2(b, c))
    };

    let t3_handle = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t3#1136", move || t3(a, c))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
