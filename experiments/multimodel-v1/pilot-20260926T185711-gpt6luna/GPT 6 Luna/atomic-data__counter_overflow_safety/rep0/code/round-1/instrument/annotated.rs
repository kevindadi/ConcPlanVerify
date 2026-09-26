mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn w2(c: Arc<Mutex<i32>>) {
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0_i32));

    let worker1 = {
        let c = Arc::clone(&m);
        cir_trace::spawn("w1", move || w1(c))
    };
    let worker2 = {
        let c = Arc::clone(&m);
        cir_trace::spawn("w2", move || w2(c))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done={}", *m.lock().unwrap());
 cir_trace::finish();}
