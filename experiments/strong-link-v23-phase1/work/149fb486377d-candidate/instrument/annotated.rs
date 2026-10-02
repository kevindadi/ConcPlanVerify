mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    1
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#363", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#401", ()));

    let worker_t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#506", move || t1(a, b))
    };

    let worker_t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#640", move || t2(a, b))
    };

    let result_t1 = worker_t1.join().unwrap();
    let result_t2 = worker_t2.join().unwrap();

    println!("DONE t1={} t2={}", result_t1, result_t2);
 cir_trace::finish();}
