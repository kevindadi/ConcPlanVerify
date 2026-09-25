mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() -> i32 {
    // Sequential helper routine performing only local computation.
    let mut x = 0;
    for i in 0..10 {
        x += i;
    }
    x
}

fn worker(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    let local = compute();
    let mut guard = m.lock().unwrap();
    let mut a = acc.lock().unwrap();
    *a += local;
    drop(a);
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = cir_trace::spawn("worker", move || worker(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = cir_trace::spawn("worker", move || worker(m2, acc2));

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
