mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(_a_guard);
    let _ = work;
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(b_guard);
    drop(_a_guard);
    let _ = work;
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#521", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#559", ()));

    let t1_a = Arc::clone(&a);
    let t1_b = Arc::clone(&b);
    let worker1 = cir_trace::spawn("t1#650", move || t1(t1_a, t1_b));

    let t2_a = Arc::clone(&a);
    let t2_b = Arc::clone(&b);
    let worker2 = cir_trace::spawn("t2#770", move || t2(t2_a, t2_b));

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
