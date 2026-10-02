mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn x1(a: &Mutex<()>, b: &Mutex<()>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn x2(a: &Mutex<()>, b: &Mutex<()>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn outer(a: &Mutex<()>, b: &Mutex<()>) {
    thread::scope(|scope| {
        let x1_worker = scope.spawn(|| x1(a, b));
        let x2_worker = scope.spawn(|| x2(a, b));

        x1_worker.join().unwrap();
        x2_worker.join().unwrap();
    });
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0#532", ());
    let b = Mutex::new_named("b_mutex0#560", ());

    thread::scope(|scope| {
        let outer_worker = scope.spawn(|| outer(&a, &b));
        outer_worker.join().unwrap();
    });

    println!("DONE done=1");
 cir_trace::finish();}
