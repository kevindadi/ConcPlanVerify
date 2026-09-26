mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>, work: Arc<AtomicUsize>, completed: Arc<AtomicUsize>) {
    let a_permit = a.acquire();
    let b_permit = b.acquire();

    work.fetch_add(1, Ordering::SeqCst);

    b_permit.release();
    a_permit.release();
    completed.fetch_add(1, Ordering::SeqCst);
}

fn t2(b: Arc<Semaphore>, c: Arc<Semaphore>, work: Arc<AtomicUsize>, completed: Arc<AtomicUsize>) {
    let b_permit = b.acquire();
    let c_permit = c.acquire();

    work.fetch_add(1, Ordering::SeqCst);

    c_permit.release();
    b_permit.release();
    completed.fetch_add(1, Ordering::SeqCst);
}

fn t3(a: Arc<Semaphore>, c: Arc<Semaphore>, work: Arc<AtomicUsize>, completed: Arc<AtomicUsize>) {
    let a_permit = a.acquire();
    let c_permit = c.acquire();

    work.fetch_add(1, Ordering::SeqCst);

    c_permit.release();
    a_permit.release();
    completed.fetch_add(1, Ordering::SeqCst);
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);

    let work = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let first = {
        let (a, b, work, completed) = (a.clone(), b.clone(), work.clone(), completed.clone());
        cir_trace::spawn("t1", move || t1(a, b, work, completed))
    };
    let second = {
        let (b, c, work, completed) = (b.clone(), c.clone(), work.clone(), completed.clone());
        cir_trace::spawn("t2", move || t2(b, c, work, completed))
    };
    let third = {
        let (a, c, work, completed) = (a.clone(), c.clone(), work.clone(), completed.clone());
        cir_trace::spawn("t3", move || t3(a, c, work, completed))
    };

    first.join().unwrap();
    second.join().unwrap();
    third.join().unwrap();

    let done = if completed.load(Ordering::SeqCst) == 3 { 1 } else { 0 };
    println!("DONE done={done}");
 cir_trace::finish();}
