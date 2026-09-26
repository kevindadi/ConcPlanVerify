mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", ());
    let b = Mutex::new_named("b_mutex0", ());
    let c = Mutex::new_named("c_mutex0", ());

    thread::scope(|s| {
        s.spawn(|| t1(&a, &b));
        s.spawn(|| t2(&b, &c));
        s.spawn(|| t3(&a, &c));
    });

    println!("DONE done=1");
 cir_trace::finish();}

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_a);
}
