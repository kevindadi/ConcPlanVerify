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

    let done = m.lock().unwrap().c;
    println!("DONE done={done}");
 cir_trace::finish();}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut cur: i32 = 0;
    cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut cur: i32 = 0;
    cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
    drop(guard);
}
