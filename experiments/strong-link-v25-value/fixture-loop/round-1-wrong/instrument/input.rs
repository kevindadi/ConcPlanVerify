// NEGATIVE control: the counter really ends at 1 (only w1 increments), while the
// program prints the "correct" terminal text. The var_eq goal c == 2 must NOT be
// satisfied; the printed line is not evidence of the internal value.
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicI32>) {
    loop {
        let observed = c.load(Ordering::SeqCst);
        let old = c.compare_exchange(observed, observed + 1, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or_else(|old| old);
        if old == observed { return; }
    }
}
fn w2(_c: Arc<AtomicI32>) {}

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
