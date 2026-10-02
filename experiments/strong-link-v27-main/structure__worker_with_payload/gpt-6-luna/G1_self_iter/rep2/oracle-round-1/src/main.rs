mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering};

fn compute() -> usize {
    let mut value = 0usize;
    for i in 0..1_000 {
        value = value.wrapping_add(i);
    }
    value
}

fn w1(m: Arc<Mutex<()>>, acc: Arc<AtomicUsize>) {
    let guard = m.lock().unwrap();
    let _work = compute();
    acc.fetch_add(1, Ordering::Relaxed);
    drop(guard);
}

fn w2(m: Arc<Mutex<()>>, acc: Arc<AtomicUsize>) {
    let guard = m.lock().unwrap();
    let _work = compute();
    acc.fetch_add(1, Ordering::Relaxed);
    drop(guard);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#594", ()));
    let acc = Arc::new(AtomicUsize::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let h1 = cir_trace::spawn("w1#725", move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let h2 = cir_trace::spawn("w2#843", move || w2(m2, acc2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = usize::from(acc.load(Ordering::Relaxed) == 2);
    println!("DONE done={done}");
 cir_trace::finish();}
