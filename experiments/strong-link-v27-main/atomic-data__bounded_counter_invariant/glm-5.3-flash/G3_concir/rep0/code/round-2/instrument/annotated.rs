mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn w1(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("res_mutex0#313", 0));

    let m1 = Arc::clone(&m);
    let h1 = cir_trace::spawn("w1#365", move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = cir_trace::spawn("w2#462", move || {
        w2(&m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
