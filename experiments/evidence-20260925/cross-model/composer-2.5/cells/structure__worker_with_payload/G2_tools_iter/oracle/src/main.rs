mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn compute() {
    let mut x = 0u64;
    for i in 0..512 {
        x = x.wrapping_mul(3).wrapping_add(i);
    }
    let _ = x;
}

fn w1(m: Arc<Semaphore>, acc: Arc<Mutex<i32>>) {
    let permit = m.acquire();
    compute();
    *acc.lock().unwrap() = 1;
    drop(permit);
}

fn w2(m: Arc<Semaphore>, acc: Arc<Mutex<i32>>) {
    let permit = m.acquire();
    compute();
    *acc.lock().unwrap() = 1;
    drop(permit);
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&m);
    let a1 = Arc::clone(&acc);
    let t1 = cir_trace::spawn("w1", move || w1(m1, a1));

    let m2 = Arc::clone(&m);
    let a2 = Arc::clone(&acc);
    let t2 = cir_trace::spawn("w2", move || w2(m2, a2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
