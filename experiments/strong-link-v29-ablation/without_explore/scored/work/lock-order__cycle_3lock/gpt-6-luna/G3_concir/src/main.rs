mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let mut work = 0;
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    work += 1;
    drop(b_guard);
    drop(a_guard);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let mut work = 0;
    let b_guard = b.lock().unwrap();
    let c_guard = c.lock().unwrap();
    work += 1;
    drop(c_guard);
    drop(b_guard);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let mut work = 0;
    let a_guard = a.lock().unwrap();
    let c_guard = c.lock().unwrap();
    work += 1;
    drop(c_guard);
    drop(a_guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#681", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#719", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#757", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = crate::cir_trace::spawn("t1#846", move || t1(a1, b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let t2_handle = crate::cir_trace::spawn("t2#960", move || t2(b2, c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let t3_handle = crate::cir_trace::spawn("t3#1074", move || t3(a3, c3));

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
