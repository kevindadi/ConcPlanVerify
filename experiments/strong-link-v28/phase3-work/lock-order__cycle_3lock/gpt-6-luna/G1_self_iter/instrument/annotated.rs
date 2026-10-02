mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#805", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _b_guard = b.lock().unwrap();
    let _c_guard = c.lock().unwrap();
    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#805", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>, work: Arc<AtomicUsize>) {
    let _a_guard = a.lock().unwrap();
    let _c_guard = c.lock().unwrap();
    { let __cpv = work.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("work#805", (work.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#704", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#742", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#780", ()));
    let work = Arc::new(AtomicUsize::new(0));

    let h1 = {
        let (a, b, work) = (Arc::clone(&a), Arc::clone(&b), Arc::clone(&work));
        cir_trace::spawn("t1#940", move || t1(a, b, work))
    };
    let h2 = {
        let (b, c, work) = (Arc::clone(&b), Arc::clone(&c), Arc::clone(&work));
        cir_trace::spawn("t2#1088", move || t2(b, c, work))
    };
    let h3 = {
        let (a, c, work) = (Arc::clone(&a), Arc::clone(&c), Arc::clone(&work));
        cir_trace::spawn("t3#1236", move || t3(a, c, work))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    assert_eq!(work.load(Ordering::Relaxed), 3);
    println!("DONE done=1");
 cir_trace::finish();}
