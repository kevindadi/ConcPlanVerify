mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0", ());
    let b = Mutex::new_named("b_mutex0", ());

    let (v1, v2) = thread::scope(|scope| {
        let h1 = scope.spawn(|| t1(&a, &b));
        let h2 = scope.spawn(|| t2(&a, &b));
        (h1.join().unwrap(), h2.join().unwrap())
    });

    println!("DONE t1={v1} t2={v2}");
 cir_trace::finish();}
