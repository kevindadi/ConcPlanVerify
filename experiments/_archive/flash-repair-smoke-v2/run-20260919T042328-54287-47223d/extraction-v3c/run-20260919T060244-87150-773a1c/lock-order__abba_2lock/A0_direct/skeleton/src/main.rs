use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    cir_trace::ev("t0", "s1");
    let w1 = thread::spawn(move || {
        cir_trace::ev("t0", "s6");
        let ga = a1.lock().unwrap();
        cir_trace::ev("t0", "s7");
        let gb = b1.lock().unwrap();
        cir_trace::ev("t0", "s8");
        drop(gb);
        cir_trace::ev("t0", "s9");
        drop(ga);
        cir_trace::ev("t0", "s10");
    });

    let (a2, b2) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    cir_trace::ev("t0", "s2");
    let w2 = thread::spawn(move || {
        cir_trace::ev("t0", "s11");
        let ga = a2.lock().unwrap();
        cir_trace::ev("t0", "s12");
        let gb = b2.lock().unwrap();
        cir_trace::ev("t0", "s13");
        drop(gb);
        cir_trace::ev("t0", "s14");
        drop(ga);
        cir_trace::ev("t0", "s15");
    });

    cir_trace::ev("t0", "s3");
    w1.join().unwrap();
    cir_trace::ev("t0", "s4");
    w2.join().unwrap();
    cir_trace::ev("t0", "s5");
    cir_trace::finish();
}