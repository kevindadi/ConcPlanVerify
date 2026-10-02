mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#413", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#451", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = cir_trace::spawn("w1#538", move || w1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = cir_trace::spawn("w2#650", move || w2(a2, b2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
