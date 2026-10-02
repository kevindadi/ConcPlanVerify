mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

fn compute() -> usize {
    (1..=10).sum()
}

fn w1(m: Arc<Mutex<()>>, acc: Arc<AtomicUsize>) {
    let guard = m.lock().unwrap();
    { let __cpv = acc.fetch_add(compute(), Ordering::SeqCst); cir_trace::record_value("acc#507", (acc.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    drop(guard);
}

fn w2(m: Arc<Mutex<()>>, acc: Arc<AtomicUsize>) {
    let guard = m.lock().unwrap();
    { let __cpv = acc.fetch_add(compute(), Ordering::SeqCst); cir_trace::record_value("acc#507", (acc.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#483", ()));
    let acc = Arc::new(AtomicUsize::new(0));

    let t1 = cir_trace::spawn("w1#552", {
        let m = Arc::clone(&m);
        let acc = Arc::clone(&acc);
        move || w1(m, acc)
    });

    let t2 = cir_trace::spawn("w2#685", {
        let m = Arc::clone(&m);
        let acc = Arc::clone(&acc);
        move || w2(m, acc)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
