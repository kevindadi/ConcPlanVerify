mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute() {
    // sequential helper routine: only local computation
    let mut x = 0u64;
    for i in 0..1000 {
        x = x.wrapping_add(i);
    }
    std::hint::black_box(x);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0u32));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = cir_trace::spawn("compute", move || {
        let _guard = m1.lock().unwrap();
        compute();
        let mut a = acc1.lock().unwrap();
        *a += 1;
    });

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = cir_trace::spawn("compute", move || {
        let _guard = m2.lock().unwrap();
        compute();
        let mut a = acc2.lock().unwrap();
        *a += 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
