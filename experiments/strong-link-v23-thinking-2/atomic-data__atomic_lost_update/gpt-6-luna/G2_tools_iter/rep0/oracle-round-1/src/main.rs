mod cir_trace;
use std::sync::{atomic::{AtomicUsize, Ordering};

fn increment(c: &AtomicUsize) {
    let mut observed = c.load(Ordering::SeqCst);

    loop {
        match c.compare_exchange(
            observed,
            observed + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => break,
            Err(actual) => observed = actual,
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

    let worker1 = cir_trace::spawn("w1#583", {
        let c = Arc::clone(&c);
        move || w1(c)
    });
    let worker2 = cir_trace::spawn("w2#684", {
        let c = Arc::clone(&c);
        move || w2(c)
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
