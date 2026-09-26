use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let c = Arc::new(AtomicI32::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);
    let t1 = thread::spawn(move || w1(c1));
    let t2 = thread::spawn(move || w2(c2));
    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}

fn w1(c: Arc<AtomicI32>) {
    loop {
        let old = c.load(Ordering::SeqCst);
        if old == 0 {
            let prev = c
                .compare_exchange(old, 1, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap_or_else(|v| v);
            if prev == old {
                return;
            }
        } else if old == 1 {
            let prev = c
                .compare_exchange(old, 2, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap_or_else(|v| v);
            if prev == old {
                return;
            }
        }
    }
}

fn w2(c: Arc<AtomicI32>) {
    loop {
        let old = c.load(Ordering::SeqCst);
        if old == 0 {
            let prev = c
                .compare_exchange(old, 1, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap_or_else(|v| v);
            if prev == old {
                return;
            }
        } else if old == 1 {
            let prev = c
                .compare_exchange(old, 2, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap_or_else(|v| v);
            if prev == old {
                return;
            }
        }
    }
}
