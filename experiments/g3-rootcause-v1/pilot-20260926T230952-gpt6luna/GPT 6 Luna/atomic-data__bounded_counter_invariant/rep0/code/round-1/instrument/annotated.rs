mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    *c += 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));

    let worker1 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w1", move || w1(m))
    };
    let worker2 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w2", move || w2(m))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
