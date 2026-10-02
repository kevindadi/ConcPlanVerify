mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(guard_b);
    drop(guard_a);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#481", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#519", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = crate::cir_trace::spawn("t1#606", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = crate::cir_trace::spawn("t2#718", move || t2(a2, b2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE t1=1 t2=1");
 crate::cir_trace::finish();}
