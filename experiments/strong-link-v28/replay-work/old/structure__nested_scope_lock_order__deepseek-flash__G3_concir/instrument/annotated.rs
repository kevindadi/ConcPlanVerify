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
    let a_x1 = Arc::clone(&a);
    let b_x1 = Arc::clone(&b);
    let h_x1 = cir_trace::spawn("x1#455", move || x1(a_x1, b_x1));

    let a_x2 = Arc::clone(&a);
    let b_x2 = Arc::clone(&b);
    let h_x2 = cir_trace::spawn("x2#572", move || x2(a_x2, b_x2));

    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#707", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#745", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);
    let h1 = cir_trace::spawn("outer#837", move || outer(a_outer, b_outer));

    h1.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
