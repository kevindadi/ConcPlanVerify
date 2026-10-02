mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let x1_handle = crate::cir_trace::spawn("x1#496", move || x1(a1, b1));
    let x2_handle = crate::cir_trace::spawn("x2#551", move || x2(a, b));

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#690", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#728", ()));

    let outer_handle = crate::cir_trace::spawn("outer#762", move || outer(a, b));
    outer_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
