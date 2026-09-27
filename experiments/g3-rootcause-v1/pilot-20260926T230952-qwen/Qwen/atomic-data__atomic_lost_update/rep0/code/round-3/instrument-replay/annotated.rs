mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn w1(c: Arc<AtomicUsize>) {
    loop {
        let old_val = c.load(Ordering::SeqCst);
        let desired = old_val + 1;
        match c.compare_exchange(old_val, desired, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(observed) => {
                if observed == old_val {
                    return;
                }
            }
            Err(_) => {}
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    loop {
        let old_val = c.load(Ordering::SeqCst);
        let desired = old_val + 1;
        match c.compare_exchange(old_val, desired, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(observed) => {
                if observed == old_val {
                    return;
                }
            }
            Err(_) => {}
        }
    }
}

fn supervisor(c: Arc<AtomicUsize>) {
    let h1 = cir_trace::spawn("w1", {
        let c_clone = c.clone();
        move || w1(c_clone)
    });
    let h2 = cir_trace::spawn("w2", {
        let c_clone = c.clone();
        move || w2(c_clone)
    });
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    supervisor(c.clone());
    println!("DONE done=1");
 cir_trace::finish();}
