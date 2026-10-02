use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI32) {
    loop {
        let exp = c.load(Ordering::SeqCst);
        let obs = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst);
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn w2(c: &AtomicI32) {
    loop {
        let exp = c.load(Ordering::SeqCst);
        let obs = c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst);
        match obs {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

fn main() {
    let c = Arc::new(AtomicI32::new(0));
    let c_w1 = Arc::clone(&c);
    let c_w2 = Arc::clone(&c);
    let h1 = thread::spawn(move || w1(&c_w1));
    let h2 = thread::spawn(move || w2(&c_w2));
    h1.join().unwrap();
    h2.join().unwrap();
    let done = c.load(Ordering::SeqCst);
    println!("DONE done={}", done / 2);
}
