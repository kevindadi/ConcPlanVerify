mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

fn increment(c: &AtomicUsize) {
    let mut current = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#699", (__cpv) as i64); __cpv };

    loop {
        let next = current + 1;
        match { let __cpv = c.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#699", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

fn w1(c: Arc<AtomicUsize>, tx: mpsc::Sender<()>) {
    increment(&c);
    cir_trace::record("channel_send", "tx"); tx.send(()).expect("w2 receiver should still be available");
}

fn w2(c: Arc<AtomicUsize>, rx: mpsc::Receiver<()>) {
    cir_trace::record("channel_recv", "rx"); rx.recv().expect("w1 should signal after incrementing");
    increment(&c);
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();

    let worker1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w1#827", move || w1(c, tx))
    };

    let worker2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("w2#928", move || w2(c, rx))
    };

    worker1.join().expect("w1 panicked");
    worker2.join().expect("w2 panicked");

    assert_eq!(c.load(Ordering::SeqCst), 2);
    println!("DONE done=1");
 cir_trace::finish();}
