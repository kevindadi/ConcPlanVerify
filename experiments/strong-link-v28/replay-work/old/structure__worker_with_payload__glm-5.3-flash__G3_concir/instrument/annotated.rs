mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn compute() {
    let mut tmp = 0;
    tmp = tmp + 1;
    let _ = tmp;
}

fn w1(m: &Mutex<()>, acc: &Mutex<i64>) {
    let _guard = m.lock().unwrap();
    compute();
    {
        let mut a = acc.lock().unwrap();
        *a = 1;
    }
    drop(_guard);
}

fn w2(m: &Mutex<()>, acc: &Mutex<i64>) {
    let _guard = m.lock().unwrap();
    compute();
    {
        let mut a = acc.lock().unwrap();
        *a = 1;
    }
    drop(_guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#509", ()));
    let acc = Arc::new(Mutex::new_named("acc_mutex0#549", 0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let h1 = cir_trace::spawn("w1#634", move || {
        w1(&m1, &acc1);
    });

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let h2 = cir_trace::spawn("w2#771", move || {
        w2(&m2, &acc2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
