mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let first = a.lock().unwrap();
    let second = b.lock().unwrap();

    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#901", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };

    drop(second);
    drop(first);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let first = b.lock().unwrap();
    let second = c.lock().unwrap();

    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#901", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };

    drop(second);
    drop(first);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let first = a.lock().unwrap();
    let second = c.lock().unwrap();

    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#901", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };

    drop(second);
    drop(first);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#800", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#838", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#876", ()));
    let work = Arc::new(AtomicUsize::new(0));

    let worker1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let work = Arc::clone(&work);
        cir_trace::spawn("t1#1063", move || t1(a, b, work))
    };

    let worker2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        let work = Arc::clone(&work);
        cir_trace::spawn("t2#1239", move || t2(b, c, work))
    };

    let worker3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        let work = Arc::clone(&work);
        cir_trace::spawn("t3#1415", move || t3(a, c, work))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();
    worker3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
