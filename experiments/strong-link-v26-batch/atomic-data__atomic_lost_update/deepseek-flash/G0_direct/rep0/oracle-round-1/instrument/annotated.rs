mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[allow(non_upper_case_globals)]
static c: AtomicUsize = AtomicUsize::new(0);

fn w1() {
    loop {
        let old = c.load(Ordering::SeqCst);
        match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2() {
    loop {
        let old = c.load(Ordering::SeqCst);
        match c.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#628", w1);
    let h2 = cir_trace::spawn("h2#660", w2);

    h1.join().unwrap();
    h2.join().unwrap();

    assert_eq!(c.load(Ordering::SeqCst), 2);

    println!("DONE done=1");
 cir_trace::finish();}
