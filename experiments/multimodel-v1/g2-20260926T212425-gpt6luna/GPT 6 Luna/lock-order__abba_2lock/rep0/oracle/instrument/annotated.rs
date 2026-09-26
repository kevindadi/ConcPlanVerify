mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>, result: Arc<AtomicUsize>) {
    let permit_a = a.acquire();
    let permit_b = b.acquire();

    result.fetch_add(1, Ordering::SeqCst);

    drop(permit_b);
    drop(permit_a);
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>, result: Arc<AtomicUsize>) {
    let permit_a = a.acquire();
    let permit_b = b.acquire();

    result.fetch_add(1, Ordering::SeqCst);

    drop(permit_b);
    drop(permit_a);
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let result_t1 = Arc::new(AtomicUsize::new(0));
    let result_t2 = Arc::new(AtomicUsize::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let result_t1_worker = Arc::clone(&result_t1);
    let worker_t1 = cir_trace::spawn("t1", move || t1(a1, b1, result_t1_worker));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let result_t2_worker = Arc::clone(&result_t2);
    let worker_t2 = cir_trace::spawn("t2", move || t2(a2, b2, result_t2_worker));

    worker_t1.join().unwrap();
    worker_t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        result_t1.load(Ordering::SeqCst),
        result_t2.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
