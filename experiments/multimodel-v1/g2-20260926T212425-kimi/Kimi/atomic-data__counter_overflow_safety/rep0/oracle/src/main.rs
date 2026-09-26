mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(m: Arc<Semaphore>, c: Arc<AtomicI32>) {
    let permit = m.acquire();
    if c.load(Ordering::SeqCst) < 1 {
        c.store(c.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
    }
    drop(permit);
}

fn w2(m: Arc<Semaphore>, c: Arc<AtomicI32>) {
    let permit = m.acquire();
    if c.load(Ordering::SeqCst) < 1 {
        c.store(c.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
    }
    drop(permit);
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let c = Arc::new(AtomicI32::new(0));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("w1", move || w1(m1, c1));

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("w2", move || w2(m2, c2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done={}", c.load(Ordering::SeqCst));
 cir_trace::finish();}
