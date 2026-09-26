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
    let h1 = std::thread::spawn({
        let c = c.clone();
        move || w1(c)
    });
    let h2 = std::thread::spawn({
        let c = c.clone();
        move || w2(c)
    });
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let c = Arc::new(AtomicUsize::new(0));
    supervisor(c.clone());
    println!("DONE done=1");
}
