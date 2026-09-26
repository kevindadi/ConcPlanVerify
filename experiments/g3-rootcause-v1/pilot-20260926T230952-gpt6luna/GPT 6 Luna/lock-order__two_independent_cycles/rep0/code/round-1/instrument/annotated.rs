mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let guard_c = c.lock().unwrap();
    let guard_d = d.lock().unwrap();
    drop(guard_d);
    drop(guard_c);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let guard_c = c.lock().unwrap();
    let guard_d = d.lock().unwrap();
    drop(guard_d);
    drop(guard_c);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1", move || t1(a, b))
    };
    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2", move || t2(a, b))
    };
    let t3_handle = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3", move || t3(c, d))
    };
    let t4_handle = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4", move || t4(c, d))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();
    t4_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
