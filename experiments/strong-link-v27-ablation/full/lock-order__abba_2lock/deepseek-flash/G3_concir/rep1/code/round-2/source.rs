use std::sync::{Arc, Mutex};
use std::thread;

/// CIR function main::t1
fn t1(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    // mutex_lock main::a
    let g1 = a.lock().unwrap();
    // mutex_lock main::b
    let g2 = b.lock().unwrap();
    // mutex_unlock main::b
    drop(g2);
    // mutex_unlock main::a
    drop(g1);
    // return {}
    1
}

/// CIR function main::t2
fn t2(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    // mutex_lock main::a
    let g1 = a.lock().unwrap();
    // mutex_lock main::b
    let g2 = b.lock().unwrap();
    // mutex_unlock main::b
    drop(g2);
    // mutex_unlock main::a
    drop(g1);
    // return {}
    1
}

/// CIR function main::main
fn main() {
    // Shared resources of module main: mutex a, mutex b.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // spawn main::t1 with handle h1
    let a_h1 = Arc::clone(&a);
    let b_h1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(&a_h1, &b_h1));

    // spawn main::t2 with handle h2
    let a_h2 = Arc::clone(&a);
    let b_h2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(&a_h2, &b_h2));

    // join h1
    let r1 = h1.join().unwrap();
    // join h2
    let r2 = h2.join().unwrap();

    // return {}
    println!("DONE t1={} t2={}", r1, r2);
}
