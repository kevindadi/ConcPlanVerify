mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = c.load(Ordering::SeqCst);

    loop {
        match c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return,
            Err(actual) => current = actual,
        }
    }
}

fn w1(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn w2(c: Arc<AtomicUsize>) {
    increment(&c);
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let worker1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w1#582", move || w1(c))
    };
    let worker2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w2#678", move || w2(c))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
