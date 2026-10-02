mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = a.clone();
    let b1 = b.clone();
    let h1 = crate::cir_trace::spawn("x1#447", move || x1(a1, b1));
    let a2 = a.clone();
    let b2 = b.clone();
    let h2 = crate::cir_trace::spawn("x2#543", move || x2(a2, b2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#669", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#707", ()));
    let a0 = a.clone();
    let b0 = b.clone();
    let h = crate::cir_trace::spawn("outer#777", move || outer(a0, b0));
    h.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
