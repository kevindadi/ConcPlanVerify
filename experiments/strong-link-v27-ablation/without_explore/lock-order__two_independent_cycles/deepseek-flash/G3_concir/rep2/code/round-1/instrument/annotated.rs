mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#163", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#201", ()));

    let t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#299", move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        })
    };

    let t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#511", move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        })
    };

    let t3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3#723", move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        })
    };

    let t4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4#935", move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        })
    };

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
