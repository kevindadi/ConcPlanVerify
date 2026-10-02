mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// CIR function main::t1
fn t1(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    // s1: mutex_lock main::a
    let guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let guard_b = b.lock().unwrap();
    // s3: assign_local x = 1
    let x: i32 = 1;
    // s4: mutex_unlock main::b
    drop(guard_b);
    // s5: mutex_unlock main::a
    drop(guard_a);
    // s6: return
    x
}

// CIR function main::t2
fn t2(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    // s1: mutex_lock main::a
    let guard_a = a.lock().unwrap();
    // s2: mutex_lock main::b
    let guard_b = b.lock().unwrap();
    // s3: assign_local x = 1
    let x: i32 = 1;
    // s4: mutex_unlock main::b
    drop(guard_b);
    // s5: mutex_unlock main::a
    drop(guard_a);
    // s6: return
    x
}

// CIR function main::main
fn main() { cir_trace::init();
    let a: Arc<Mutex<()>> = Arc::new(Mutex::new_named("res_mutex0#896", ()));
    let b: Arc<Mutex<()>> = Arc::new(Mutex::new_named("res_mutex0#950", ()));

    // s1: scope { main::t1, main::t2 }
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#1072", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#1181", move || t2(&a2, &b2));

    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}
