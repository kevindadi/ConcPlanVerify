mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_handle = cir_trace::spawn("x1#342", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || x1(a, b)
    });

    let x2_handle = cir_trace::spawn("x2#476", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || x2(a, b)
    });

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#695", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#733", ()));

    let outer_handle = cir_trace::spawn("outer#767", move || outer(a, b));
    outer_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
