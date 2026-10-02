mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn w1(a: &Mutex<()>, b: &Mutex<()>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();

    // Work is performed while both mutexes are held.
}

fn w2(a: &Mutex<()>, b: &Mutex<()>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();

    // Work is performed while both mutexes are held.
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0#415", ());
    let b = Mutex::new_named("b_mutex0#443", ());

    thread::scope(|scope| {
        let h1 = scope.spawn(|| w1(&a, &b));
        let h2 = scope.spawn(|| w2(&a, &b));

        h1.join().unwrap();
        h2.join().unwrap();
    });

    println!("DONE done=1");
 cir_trace::finish();}
