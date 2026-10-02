mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#163", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#201", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("h1#283", move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("h2#501", move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        drop(_gb);
        drop(_ga);
    });

    let c1 = Arc::clone(&c);
    let d1 = Arc::clone(&d);
    let h3 = cir_trace::spawn("h3#719", move || {
        let _gc = c1.lock().unwrap();
        let _gd = d1.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    let c2 = Arc::clone(&c);
    let d2 = Arc::clone(&d);
    let h4 = cir_trace::spawn("h4#937", move || {
        let _gc = c2.lock().unwrap();
        let _gd = d2.lock().unwrap();
        drop(_gd);
        drop(_gc);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
