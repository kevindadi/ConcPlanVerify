mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() -> i32 {
    let tmp = 1;
    tmp
}

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _tmp = compute();
    *guard = *guard + 1;
    drop(guard);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let _tmp = compute();
    *guard = *guard + 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#413", 0i32));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let t1 = cir_trace::spawn("w1#498", move || w1(m1));
    let t2 = cir_trace::spawn("w2#542", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
