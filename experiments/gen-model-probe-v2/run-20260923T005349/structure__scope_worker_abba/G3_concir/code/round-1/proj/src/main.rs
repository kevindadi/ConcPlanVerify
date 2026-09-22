mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn w1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn w2(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", ());
    let b = Mutex::new_named("b_mutex0", ());
    let mut done = 0;

    thread::scope(|s| {
        s.spawn(|| w1(&a, &b));
        s.spawn(|| w2(&a, &b));
    });

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
