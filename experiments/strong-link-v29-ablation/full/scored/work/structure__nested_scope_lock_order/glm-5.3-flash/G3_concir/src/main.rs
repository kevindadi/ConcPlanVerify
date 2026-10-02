mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn x2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("x1#461", move || x1(&a1, &b1));
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = crate::cir_trace::spawn("x2#569", move || x2(&a2, &b2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#697", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#735", ()));
    let a_o = Arc::clone(&a);
    let b_o = Arc::clone(&b);
    let h = crate::cir_trace::spawn("outer#817", move || outer(a_o, b_o));
    h.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
