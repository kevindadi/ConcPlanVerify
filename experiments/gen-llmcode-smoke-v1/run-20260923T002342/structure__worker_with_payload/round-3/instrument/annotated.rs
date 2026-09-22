mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w2() -> i32 {
    let local = 1;
    local
}

fn w1(m: &Mutex<i32>, acc: &Mutex<i32>) {
    {
        let _guard = m.lock().unwrap();
        let _ = w2();
        let tmp = *acc.lock().unwrap();
        let tmp2 = tmp + 1;
        *acc.lock().unwrap() = tmp2;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let h1 = cir_trace::spawn("h1", move || {
        w1(&m1, &acc1);
    });

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let h2 = cir_trace::spawn("h2", move || {
        w1(&m2, &acc2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
