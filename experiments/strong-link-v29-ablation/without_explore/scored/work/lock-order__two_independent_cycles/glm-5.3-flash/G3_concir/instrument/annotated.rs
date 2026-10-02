mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn t1(a: &Mutex<i32>, b: &Mutex<i32>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(a: &Mutex<i32>, b: &Mutex<i32>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t3(c: &Mutex<i32>, d: &Mutex<i32>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn t4(c: &Mutex<i32>, d: &Mutex<i32>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#626", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0#663", 0));
    let c = Arc::new(Mutex::new_named("c_mutex0#700", 0));
    let d = Arc::new(Mutex::new_named("d_mutex0#737", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("t1#818", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = crate::cir_trace::spawn("t2#932", move || t2(&a2, &b2));

    let c1 = Arc::clone(&c);
    let d1 = Arc::clone(&d);
    let h3 = crate::cir_trace::spawn("t3#1046", move || t3(&c1, &d1));

    let c2 = Arc::clone(&c);
    let d2 = Arc::clone(&d);
    let h4 = crate::cir_trace::spawn("t4#1160", move || t4(&c2, &d2));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    let done = *a.lock().unwrap();
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
