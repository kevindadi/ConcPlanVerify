mod cir_trace;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;

fn cas_add(c: &AtomicIsize) {
    loop {
        let old = c.load(Ordering::SeqCst);
        if c
            .compare_exchange_weak(old, old + 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            break;
        }
        // failed attempt: retry, never abandon
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicIsize::new(0));

    let c1 = Arc::clone(&c);
    let h1 = cir_trace::spawn("cas_add#467", move || {
        cas_add(&c1);
    });

    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("cas_add#569", move || {
        cas_add(&c2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
