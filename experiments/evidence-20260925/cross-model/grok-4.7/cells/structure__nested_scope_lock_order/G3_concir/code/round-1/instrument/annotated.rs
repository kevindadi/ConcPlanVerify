mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0i32));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let outer_handle = cir_trace::spawn("outer", move || {
        outer(a_outer, b);
    });
    outer_handle.join().unwrap();

    let done = *a.lock().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}

fn outer(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let x1_handle = cir_trace::spawn("x1", move || {
        x1(a1, b1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let x2_handle = cir_trace::spawn("x2", move || {
        x2(a2, b2);
    });

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();

    {
        let mut done = a.lock().unwrap();
        *done = 1;
    }
}

fn x1(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn x2(a: Arc<Mutex<i32>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}
