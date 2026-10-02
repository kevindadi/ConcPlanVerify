mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let x1_a = Arc::clone(&a);
    let x1_b = Arc::clone(&b);
    let x2_a = Arc::clone(&a);
    let x2_b = Arc::clone(&b);

    let task_x1 = cir_trace::spawn("x1#489", move || x1(x1_a, x1_b));
    let task_x2 = cir_trace::spawn("x2#546", move || x2(x2_a, x2_b));

    task_x1.join().unwrap();
    task_x2.join().unwrap();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#687", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#725", ()));

    let worker = cir_trace::spawn("outer#753", move || outer(a, b));
    worker.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
