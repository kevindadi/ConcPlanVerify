mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn w1(m: Arc<Mutex<i32>>) {
    {
        let mut acc = m.lock().unwrap();
        compute();
        let tmp = *acc;
        let tmp = tmp + 1;
        *acc = tmp;
    }
}

fn w2(m: Arc<Mutex<i32>>) {
    {
        let mut acc = m.lock().unwrap();
        compute();
        let tmp = *acc;
        let tmp = tmp + 1;
        *acc = tmp;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1", move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2", move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
