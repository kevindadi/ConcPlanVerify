mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work is performed while both locks are held.
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work is performed while both locks are held.
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#461", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#499", ()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#604", move || t1(a, b))
    };

    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#738", move || t2(a, b))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
