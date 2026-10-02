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

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#409", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#447", ()));

    let worker1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("t1#550", move || t1(a, b))
    };
    let worker2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("t2#681", move || t2(a, b))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE t1=1 t2=1");
 crate::cir_trace::finish();}
