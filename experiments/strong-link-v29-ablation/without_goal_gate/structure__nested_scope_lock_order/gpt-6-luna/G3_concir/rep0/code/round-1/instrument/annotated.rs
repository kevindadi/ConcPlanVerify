mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x1_handle = crate::cir_trace::spawn("x1#500", move || x1(x1_a, x1_b));

    let x2_handle = crate::cir_trace::spawn("x2#560", move || x2(a, b));

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#699", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#737", ()));

    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);
    let outer_handle = crate::cir_trace::spawn("outer#839", move || outer(outer_a, outer_b));

    outer_handle.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
