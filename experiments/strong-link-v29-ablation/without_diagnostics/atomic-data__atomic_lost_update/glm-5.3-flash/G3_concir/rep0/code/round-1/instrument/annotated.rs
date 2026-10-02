mod cir_trace;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

static C: AtomicI32 = AtomicI32::new(0);

fn w1() {
    loop {
        let exp = C.load(Ordering::SeqCst);
        let obs = C.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst);
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2() {
    loop {
        let exp = C.load(Ordering::SeqCst);
        let obs = C.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst);
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() { crate::cir_trace::init();
    let h1 = crate::cir_trace::spawn("w1#655", move || w1());
    let h2 = crate::cir_trace::spawn("w2#697", move || w2());
    h1.join().unwrap();
    h2.join().unwrap();
    let done = C.load(Ordering::SeqCst);
    println!("DONE done={}", done / 2);
 crate::cir_trace::finish();}
