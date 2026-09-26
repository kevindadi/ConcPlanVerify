mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1", move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = cir_trace::spawn("w2", move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = guard.c;
    tmp = tmp + 1;
    guard.c = tmp;
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = guard.c;
    tmp = tmp + 1;
    guard.c = tmp;
    drop(guard);
}
