mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#409", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#447", ()));

    let handle1 = crate::cir_trace::spawn("w1#476", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || w1(a, b)
    });
    let handle2 = crate::cir_trace::spawn("w2#607", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || w2(a, b)
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
