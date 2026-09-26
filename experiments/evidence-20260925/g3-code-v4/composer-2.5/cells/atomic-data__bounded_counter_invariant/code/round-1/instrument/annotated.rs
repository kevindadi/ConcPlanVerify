mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let m_w1 = Arc::clone(&m);
    let m_w2 = Arc::clone(&m);

    let h1 = cir_trace::spawn("w1", move || w1(m_w1));
    let h2 = cir_trace::spawn("w2", move || w2(m_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
