mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

#[allow(unused_imports)]
use concir_sync::Semaphore;

struct Shared {
    acc: i32,
}

fn compute() {
    let mut local = 0u64;
    let mut i = 0u64;
    while i < 32 {
        local = local.wrapping_add(i.wrapping_mul(17).wrapping_add(3));
        i = i.wrapping_add(1);
    }
    std::hint::black_box(local);
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut shared = m.lock().unwrap();
    compute();
    shared.acc = 1;
    drop(shared);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut shared = m.lock().unwrap();
    compute();
    shared.acc = 1;
    drop(shared);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", Shared { acc: 0 }));
    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let t1 = cir_trace::spawn("w1", move || w1(m1));
    let t2 = cir_trace::spawn("w2", move || w2(m2));
    t1.join().unwrap();
    t2.join().unwrap();
    let done = m.lock().unwrap().acc;
    println!("DONE done={done}");
 cir_trace::finish();}
