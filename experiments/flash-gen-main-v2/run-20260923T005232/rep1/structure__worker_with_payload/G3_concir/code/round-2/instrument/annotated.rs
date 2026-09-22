mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn w1(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp = acc.lock().unwrap();
        *tmp = *tmp + 1;
        let _ = &mut guard;
    }
}

fn w2(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp = acc.lock().unwrap();
        *tmp = *tmp + 1;
        let _ = &mut guard;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let h1 = cir_trace::spawn("h1", move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let h2 = cir_trace::spawn("h2", move || w2(m2, acc2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
