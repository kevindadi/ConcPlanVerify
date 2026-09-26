mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn compute(x: u64) -> u64 {
    // Sequential helper routine: only local computation.
    let mut v = x;
    let mut i = 0;
    while i < 1000 {
        v = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        i += 1;
    }
    v
}

fn worker(m: Arc<Mutex<u64>>, acc: Arc<Mutex<u64>>) {
    let mut guard = m.lock().unwrap();
    let local = compute(*guard);
    *guard = local;
    let mut a = acc.lock().unwrap();
    *a += 1;
    drop(a);
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 1u64));
    let acc = Arc::new(Mutex::new_named("acc_mutex0", 0u64));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let w1 = cir_trace::spawn("worker", move || worker(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let w2 = cir_trace::spawn("worker", move || worker(m2, acc2));

    w1.join().unwrap();
    w2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
