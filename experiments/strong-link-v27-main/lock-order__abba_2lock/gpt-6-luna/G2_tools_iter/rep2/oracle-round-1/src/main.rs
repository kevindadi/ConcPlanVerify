mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let result = 1;

    drop(guard_b);
    drop(guard_a);
    result
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let result = 1;

    drop(guard_b);
    drop(guard_a);
    result
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#493", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#531", ()));

    let handle_t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#636", move || t1(a, b))
    };

    let handle_t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#770", move || t2(a, b))
    };

    let result_t1 = handle_t1.join().unwrap();
    let result_t2 = handle_t2.join().unwrap();

    println!("DONE t1={} t2={}", result_t1, result_t2);
 cir_trace::finish();}
