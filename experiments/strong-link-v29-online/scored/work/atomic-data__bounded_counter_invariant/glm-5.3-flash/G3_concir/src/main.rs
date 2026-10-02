mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn w1(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let c = *guard;
    *guard = c + 1;
    drop(guard);
}

fn w2(m: &Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    let c = *guard;
    *guard = c + 1;
    drop(guard);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#326", 0i32));

    let m1 = Arc::clone(&m);
    let h1 = crate::cir_trace::spawn("w1#381", move || w1(&m1));

    let m2 = Arc::clone(&m);
    let h2 = crate::cir_trace::spawn("w2#461", move || w2(&m2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    let final_c = *m.lock().unwrap();
    println!("DONE done={}", final_c / 2);
 crate::cir_trace::finish();}
