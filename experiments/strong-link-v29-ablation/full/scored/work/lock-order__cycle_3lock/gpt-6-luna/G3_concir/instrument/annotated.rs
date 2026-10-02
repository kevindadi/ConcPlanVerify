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

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_a);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#570", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#608", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#646", ()));

    let handle1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("t1#749", move || t1(a, b))
    };
    let handle2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t2#880", move || t2(b, c))
    };
    let handle3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t3#1011", move || t3(a, c))
    };

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
