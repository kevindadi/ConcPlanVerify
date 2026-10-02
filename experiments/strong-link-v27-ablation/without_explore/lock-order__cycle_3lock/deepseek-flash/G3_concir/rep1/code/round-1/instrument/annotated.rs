mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#163", ()));

    // t1 needs a then b
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let h1 = cir_trace::spawn("h1#265", move || {
        let ga = a1.lock().unwrap();
        let gb = b1.lock().unwrap();
        let _x: i32 = 1;
        drop(gb);
        drop(ga);
    });

    // t2 needs b then c
    let (b2, c2) = (Arc::clone(&b), Arc::clone(&c));
    let h2 = cir_trace::spawn("h2#524", move || {
        let gb = b2.lock().unwrap();
        let gc = c2.lock().unwrap();
        let _x: i32 = 1;
        drop(gc);
        drop(gb);
    });

    // t3 needs a then c
    let (a3, c3) = (Arc::clone(&a), Arc::clone(&c));
    let h3 = cir_trace::spawn("h3#783", move || {
        let ga = a3.lock().unwrap();
        let gc = c3.lock().unwrap();
        let _x: i32 = 1;
        drop(gc);
        drop(ga);
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
