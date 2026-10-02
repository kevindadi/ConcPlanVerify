use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    let mut current = c.load(Ordering::Relaxed);
    loop {
        match c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    let mut current = c.load(Ordering::Relaxed);
    loop {
        match c.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

fn main() {
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let t1 = thread::spawn(move || w1(c1));
    let t2 = thread::spawn(move || w2(c2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
