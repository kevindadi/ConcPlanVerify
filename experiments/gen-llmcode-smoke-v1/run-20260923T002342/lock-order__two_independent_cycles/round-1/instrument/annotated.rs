mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("h1", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("h2", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("h3", move || {
        let _gc = c3.lock().unwrap();
        let _gd = d3.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("h4", move || {
        let _gc = c4.lock().unwrap();
        let _gd = d4.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
