mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x2_a = Arc::clone(&a);
    let x2_b = Arc::clone(&b);

    let x1_worker = cir_trace::spawn("x1#467", move || x1(x1_a, x1_b));
    let x2_worker = cir_trace::spawn("x2#526", move || x2(x2_a, x2_b));

    let x1_result = x1_worker.join();
    let x2_result = x2_worker.join();

    x1_result.unwrap();
    x2_result.unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#734", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#772", ()));

    let outer_a = Arc::clone(&a);
    let outer_b = Arc::clone(&b);
    let outer_worker = cir_trace::spawn("outer#874", move || outer(outer_a, outer_b));

    outer_worker.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
