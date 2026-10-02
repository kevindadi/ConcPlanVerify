mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) -> i32 {
    let mut work: i32 = 0;

    // mutex_lock main::a
    let mut ga = a.lock().unwrap();
    // mutex_lock main::b
    let mut gb = b.lock().unwrap();

    // assign_local work = work + 1
    work = work + 1;

    // mutex_unlock main::b
    drop(gb);
    // mutex_unlock main::a
    drop(ga);

    work
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) -> i32 {
    let mut work: i32 = 0;

    // mutex_lock main::a
    let mut ga = a.lock().unwrap();
    // mutex_lock main::b
    let mut gb = b.lock().unwrap();

    // assign_local work = work + 1
    work = work + 1;

    // mutex_unlock main::b
    drop(gb);
    // mutex_unlock main::a
    drop(ga);

    work
}

fn main() { crate::cir_trace::init();
    // shared resources: a (lock), b (lock)
    let a: Arc<Mutex<()>> = Arc::new(Mutex::new_named("a_mutex0#873", ()));
    let b: Arc<Mutex<()>> = Arc::new(Mutex::new_named("b_mutex0#927", ()));

    // scope { funcs: [main::t1, main::t2] }
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("t1#1054", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = crate::cir_trace::spawn("t2#1163", move || t2(&a2, &b2));

    // join both workers
    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();

    // terminal line
    println!("DONE t1={} t2={}", r1, r2);
 crate::cir_trace::finish();}
