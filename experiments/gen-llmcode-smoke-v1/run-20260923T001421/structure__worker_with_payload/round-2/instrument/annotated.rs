mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w2() {
    let local: i32 = 1;
    let _ = local;
}

fn w1(m: Arc<Mutex<i32>>) {
    {
        let mut acc = m.lock().unwrap();
        w2();
        let tmp = *acc;
        let tmp2 = tmp + 1;
        *acc = tmp2;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("h1", move || {
        w1(m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("h2", move || {
        w1(m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
