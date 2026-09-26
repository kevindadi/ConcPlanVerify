mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};


struct Counter {
    c: u8,
}

static m: Mutex<Counter> = Mutex::new_named("m_mutex0", Counter { c: 0 });

fn w1() {
    let mut guard = m.lock().unwrap();
    guard.c += 1;
    drop(guard);
}

fn w2() {
    let mut guard = m.lock().unwrap();
    guard.c += 1;
    drop(guard);
}

fn main() { cir_trace::init();
    let t1 = cir_trace::spawn("t1", w1);
    let t2 = cir_trace::spawn("t2", w2);

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
