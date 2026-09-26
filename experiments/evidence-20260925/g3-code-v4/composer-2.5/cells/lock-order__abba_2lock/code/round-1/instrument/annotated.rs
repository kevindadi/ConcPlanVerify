mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct GuardedByA {
    t1: i32,
    t2: i32,
}

fn t1(a: Arc<Mutex<GuardedByA>>, b: Arc<Mutex<()>>) {
    let mut ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    ga.t1 = 1;
    drop(_gb);
    drop(ga);
}

fn t2(a: Arc<Mutex<GuardedByA>>, b: Arc<Mutex<()>>) {
    let mut ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    ga.t2 = 1;
    drop(_gb);
    drop(ga);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", GuardedByA { t1: 0, t2: 0 }));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    let ga = a.lock().unwrap();
    println!("DONE t1={} t2={}", ga.t1, ga.t2);
 cir_trace::finish();}
