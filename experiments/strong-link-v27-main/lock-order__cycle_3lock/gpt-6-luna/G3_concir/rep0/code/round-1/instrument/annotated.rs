mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};

use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn t2(b: &Mutex<()>, c: &Mutex<()>) {
    let _b = b.lock().unwrap();
    let _c = c.lock().unwrap();
}

fn t3(a: &Mutex<()>, c: &Mutex<()>) {
    let _a = a.lock().unwrap();
    let _c = c.lock().unwrap();
}

fn main() { cir_trace::init();
    let a = Mutex::new_named("a_mutex0#386", ());
    let b = Mutex::new_named("b_mutex0#414", ());
    let c = Mutex::new_named("c_mutex0#442", ());

    thread::scope(|scope| {
        let h1 = scope.spawn(|| t1(&a, &b));
        let h2 = scope.spawn(|| t2(&b, &c));
        let h3 = scope.spawn(|| t3(&a, &c));

        h1.join().unwrap();
        h2.join().unwrap();
        h3.join().unwrap();
    });

    println!("DONE done=1");
 cir_trace::finish();}
