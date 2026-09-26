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

    thread::scope(|scope| {
        let worker1 = scope.spawn(|| w1(&a, &b));
        let worker2 = scope.spawn(|| w2(&a, &b));

        worker1.join().unwrap();
        worker2.join().unwrap();
    });

    println!("DONE done=1");
 cir_trace::finish();}
