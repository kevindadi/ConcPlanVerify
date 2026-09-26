mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let pa = a.acquire();
    let pb = b.acquire();
    done.store(1, Ordering::SeqCst);
    pb.release();
    pa.release();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let pa = a.acquire();
    let pb = b.acquire();
    done.store(1, Ordering::SeqCst);
    pb.release();
    pa.release();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let pc = c.acquire();
    let pd = d.acquire();
    done.store(1, Ordering::SeqCst);
    pd.release();
    pc.release();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let pc = c.acquire();
    let pd = d.acquire();
    done.store(1, Ordering::SeqCst);
    pd.release();
    pc.release();
}

fn main() { cir_trace::init();
    let a = Semaphore::new_named("a_semaphore0", 1);
    let b = Semaphore::new_named("b_semaphore0", 1);
    let c = Semaphore::new_named("c_semaphore0", 1);
    let d = Semaphore::new_named("d_semaphore0", 1);
    let done = Arc::new(AtomicUsize::new(0));

    let h1 = cir_trace::spawn("h1", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&done);
        move || t1(a, b, done)
    });
    let h2 = cir_trace::spawn("h2", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let done = Arc::clone(&done);
        move || t2(a, b, done)
    });
    let h3 = cir_trace::spawn("h3", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        let done = Arc::clone(&done);
        move || t3(c, d, done)
    });
    let h4 = cir_trace::spawn("h4", {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        let done = Arc::clone(&done);
        move || t4(c, d, done)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done={}", done.load(Ordering::SeqCst));
 cir_trace::finish();}
