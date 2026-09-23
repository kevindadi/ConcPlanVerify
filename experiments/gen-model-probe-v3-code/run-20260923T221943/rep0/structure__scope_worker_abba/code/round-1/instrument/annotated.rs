mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn w2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let mut done = 0;

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("w1", move || w1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("w2", move || w2(&a2, &b2));

    t1.join().unwrap();
    t2.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
