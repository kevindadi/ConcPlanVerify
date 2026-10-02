use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = c.load(Ordering::SeqCst);

    loop {
        let next = current + 1;
        match c.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn w1(c: Arc<AtomicUsize>, tx: mpsc::Sender<()>) {
    increment(&c);
    tx.send(()).expect("w2 receiver should still be available");
}

fn w2(c: Arc<AtomicUsize>, rx: mpsc::Receiver<()>) {
    rx.recv().expect("w1 should signal after incrementing");
    increment(&c);
}

fn main() {
    let c = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();

    let worker1 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w1(c, tx))
    };

    let worker2 = {
        let c = Arc::clone(&c);
        thread::spawn(move || w2(c, rx))
    };

    worker1.join().expect("w1 panicked");
    worker2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
}
