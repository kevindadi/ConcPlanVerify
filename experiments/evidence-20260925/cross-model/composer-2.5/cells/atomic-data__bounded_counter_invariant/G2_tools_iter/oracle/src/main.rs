mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(m: Arc<Semaphore>, c: Arc<AtomicI32>) {
    let _permit = m.acquire();
    let v = c.load(Ordering::Acquire);
    c.store(v + 1, Ordering::Release);
}

fn w2(m: Arc<Semaphore>, c: Arc<AtomicI32>) {
    let _permit = m.acquire();
    let v = c.load(Ordering::Acquire);
    c.store(v + 1, Ordering::Release);
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let c = Arc::new(AtomicI32::new(0));

    let m_w1 = Arc::clone(&m);
    let c_w1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1", move || w1(m_w1, c_w1));

    let m_w2 = Arc::clone(&m);
    let c_w2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2", move || w2(m_w2, c_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
