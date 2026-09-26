mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
    thread::scope(|s| {
        s.spawn(|| x1(a, b));
        s.spawn(|| x2(a, b));
    });
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let h = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("outer", move || outer(&a, &b))
    };
    h.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
