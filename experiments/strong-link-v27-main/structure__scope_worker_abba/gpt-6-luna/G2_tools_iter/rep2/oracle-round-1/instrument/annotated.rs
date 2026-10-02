mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#337", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#375", ()));

    let worker1 = cir_trace::spawn("worker1#404", w1, Arc::clone(&a), Arc::clone(&b));
    let worker2 = cir_trace::spawn("worker2#473", w2, Arc::clone(&a), Arc::clone(&b));

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
