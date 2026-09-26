mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn w1(c: Arc<AtomicUsize>, sem: Arc<Semaphore>) {
    let _permit = sem.acquire();
    loop {
        let cur = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: Arc<AtomicUsize>, sem: Arc<Semaphore>) {
    let _permit = sem.acquire();
    loop {
        let cur = c.load(Ordering::Acquire);
        match c.compare_exchange_weak(cur, cur + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    let sem = Semaphore::new_named("sem_semaphore0", 2);

    let sup = cir_trace::spawn("sup", {
        let c = Arc::clone(&c);
        let sem = Arc::clone(&sem);
        move || {
            let c1 = Arc::clone(&c);
            let sem1 = Arc::clone(&sem);
            let h1 = cir_trace::spawn("w1", move || w1(c1, sem1));

            let c2 = Arc::clone(&c);
            let sem2 = Arc::clone(&sem);
            let h2 = cir_trace::spawn("w2", move || w2(c2, sem2));

            h1.join().unwrap();
            h2.join().unwrap();
        }
    });

    sup.join().unwrap();

    debug_assert_eq!(c.load(Ordering::Acquire), 2);
    println!("DONE done=1");
 cir_trace::finish();}
