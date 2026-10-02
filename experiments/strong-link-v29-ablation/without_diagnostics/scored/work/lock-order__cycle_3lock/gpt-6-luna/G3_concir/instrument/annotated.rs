mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(b_guard);
    drop(a_guard);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let b_guard = b.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(c_guard);
    drop(b_guard);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(c_guard);
    drop(a_guard);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#705", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#743", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#781", ()));

    let t1_a = Arc::clone(&a);
    let t1_b = Arc::clone(&b);
    let handle1 = crate::cir_trace::spawn("t1#872", move || t1(t1_a, t1_b));

    let t2_b = Arc::clone(&b);
    let t2_c = Arc::clone(&c);
    let handle2 = crate::cir_trace::spawn("t2#992", move || t2(t2_b, t2_c));

    let t3_a = Arc::clone(&a);
    let t3_c = Arc::clone(&c);
    let handle3 = crate::cir_trace::spawn("t3#1112", move || t3(t3_a, t3_c));

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
