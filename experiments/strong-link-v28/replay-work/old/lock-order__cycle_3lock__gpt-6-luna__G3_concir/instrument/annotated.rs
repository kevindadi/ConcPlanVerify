mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let mut work = 0;
    work = 1;

    drop(guard_b);
    drop(guard_a);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();

    let mut work = 0;
    work = 1;

    drop(guard_c);
    drop(guard_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();

    let mut work = 0;
    work = 1;

    drop(guard_c);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#684", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#722", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#760", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = cir_trace::spawn("t1#847", move || t1(a1, b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let handle2 = cir_trace::spawn("t2#959", move || t2(b2, c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let handle3 = cir_trace::spawn("t3#1071", move || t3(a3, c3));

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
