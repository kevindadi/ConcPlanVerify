mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));

    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);
    let handle = cir_trace::spawn("outer#221", move || outer(outer_a, outer_b));

    handle.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x2_a = Arc::clone(&a);
    let x2_b = Arc::clone(&b);

    let handle1 = cir_trace::spawn("x1#522", move || x1(x1_a, x1_b));
    let handle2 = cir_trace::spawn("x2#579", move || x2(x2_a, x2_b));

    handle1.join().unwrap();
    handle2.join().unwrap();
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}
