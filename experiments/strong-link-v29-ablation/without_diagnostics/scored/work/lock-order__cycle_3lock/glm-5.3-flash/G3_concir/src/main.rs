mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    // critical work while holding both locks
    drop(_gb);
    drop(_ga);
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    // critical work while holding both locks
    drop(_gc);
    drop(_gb);
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    // critical work while holding both locks
    drop(_gc);
    drop(_ga);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#636", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#674", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#712", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("t1#794", move || t1(&a1, &b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = crate::cir_trace::spawn("t2#903", move || t2(&b2, &c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = crate::cir_trace::spawn("t3#1012", move || t3(&a3, &c3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
