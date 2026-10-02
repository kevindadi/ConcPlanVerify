mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn worker(c: Arc<AtomicUsize>) {
    loop {
        let current = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#363", (__cpv) as i64); __cpv };
        let next = current + 1;
        if { let __cpv = c.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#363", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv }.is_ok() {
            break;
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));

    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("worker#437", move || worker(c1));

    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("worker#515", move || worker(c2));

    w1.join().unwrap();
    w2.join().unwrap();

    let done = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#363", (__cpv) as i64); __cpv };
    println!("DONE done={}", done);
 cir_trace::finish();}
