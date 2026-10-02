mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = a.clone();
    let b1 = b.clone();
    let h1 = cir_trace::spawn("x1#439", move || x1(a1, b1));

    let a2 = a.clone();
    let b2 = b.clone();
    let h2 = cir_trace::spawn("x2#536", move || x2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#663", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#701", ()));

    let a_outer = a.clone();
    let b_outer = b.clone();
    let outer_h = cir_trace::spawn("outer#788", move || outer(a_outer, b_outer));

    outer_h.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
