mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn compute() -> i32 {
    let mut total = 0i32;
    let mut i = 1i32;
    while i <= 10 {
        total = total.wrapping_add(i);
        i = i.wrapping_add(1);
    }
    total / 55
}

fn w1(m: Arc<Semaphore>, acc: Arc<AtomicI32>) {
    let permit = m.acquire();
    let value = compute();
    acc.store(value, Ordering::SeqCst);
    permit.release();
}

fn w2(m: Arc<Semaphore>, acc: Arc<AtomicI32>) {
    let permit = m.acquire();
    let value = compute();
    acc.store(value, Ordering::SeqCst);
    permit.release();
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let acc = Arc::new(AtomicI32::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let t1 = cir_trace::spawn("w1", move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let t2 = cir_trace::spawn("w2", move || w2(m2, acc2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done={}", acc.load(Ordering::SeqCst));
 cir_trace::finish();}
