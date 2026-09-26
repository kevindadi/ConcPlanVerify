mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn outer(a: &Mutex<()>, b: &Mutex<()>) {
    thread::scope(|scope| {
        scope.spawn(|| x1(a, b));
        scope.spawn(|| x2(a, b));
    });
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", ());
    let b = Mutex::new_named("b_mutex0", ());

    thread::scope(|scope| {
        scope.spawn(|| outer(&a, &b));
    });

    println!("DONE done=1");
 cir_trace::finish();}
