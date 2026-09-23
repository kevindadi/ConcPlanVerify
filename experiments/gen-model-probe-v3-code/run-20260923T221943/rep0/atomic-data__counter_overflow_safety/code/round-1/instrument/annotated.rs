mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c = *c + 1;
    }
    drop(c);
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap();
    if *c < 1 {
        *c = *c + 1;
    }
    drop(c);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    let h1 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w1", move || w1(m))
    };
    let h2 = {
        let m = Arc::clone(&m);
        cir_trace::spawn("w2", move || w2(m))
    };

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
