mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::SeqCst);
        match c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    loop {
        let current = c.load(Ordering::SeqCst);
        match c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("w1#734", move || w1(c1));
    let h2 = cir_trace::spawn("w2#778", move || w2(c2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
