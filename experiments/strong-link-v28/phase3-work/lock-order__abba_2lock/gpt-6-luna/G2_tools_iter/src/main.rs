mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, count: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    count.fetch_add(1, Ordering::SeqCst);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, count: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    count.fetch_add(1, Ordering::SeqCst);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#515", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#553", ()));
    let t1_count = Arc::new(AtomicUsize::new(0));
    let t2_count = Arc::new(AtomicUsize::new(0));

    let worker_t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let count = Arc::clone(&t1_count);
        cir_trace::spawn("t1#801", move || t1(a, b, count))
    };

    let worker_t2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let count = Arc::clone(&t2_count);
        cir_trace::spawn("t2#985", move || t2(a, b, count))
    };

    worker_t1.join().unwrap();
    worker_t2.join().unwrap();

    println!(
        "DONE t1={} t2={}",
        t1_count.load(Ordering::SeqCst),
        t2_count.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
