mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: Arc<AtomicUsize>) {
    let mut expected = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#664", (__cpv) as i64); __cpv };
    loop {
        match { let __cpv = c.compare_exchange(expected, expected + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#664", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(actual) => expected = actual,
        }
    }
}

fn w2(c: Arc<AtomicUsize>) {
    let mut expected = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#664", (__cpv) as i64); __cpv };
    loop {
        match { let __cpv = c.compare_exchange(expected, expected + 1, Ordering::SeqCst, Ordering::SeqCst); if __cpv.is_ok() { cir_trace::record_value("c#664", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(actual) => expected = actual,
        }
    }
}

fn main() { cir_trace::init();
    let c = Arc::new(AtomicUsize::new(0));
    let c1 = Arc::clone(&c);
    let c2 = Arc::clone(&c);

    let h1 = cir_trace::spawn("w1#767", move || w1(c1));
    let h2 = cir_trace::spawn("w2#811", move || w2(c2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
