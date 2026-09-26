mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn compute() {
    let mut x: u64 = 1;
    for i in 0..512 {
        x = x.wrapping_mul(3).wrapping_add(i);
    }
    let _ = x;
}

fn w1(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    compute();
    *acc.lock().unwrap() = 1;
}

fn w2(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    compute();
    *acc.lock().unwrap() = 1;
}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 2);
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let t1 = cir_trace::spawn("w1", move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let t2 = cir_trace::spawn("w2", move || w2(m2, acc2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}
