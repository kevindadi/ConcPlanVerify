mod cir_trace;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn worker(c: &Arc<AtomicUsize>) {
    // R4/R5: atomic read-modify-write retry loop.
    // R7: compare_exchange is a single indivisible step; no partial update
    // is observable and no update can be silently lost.
    // R8: under any interleaving the counter only moves up, so this loop
    // can never spin forever; every retry either succeeds or another
    // worker made progress.
    loop {
        let cur = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#942", (__cpv) as i64); __cpv };
        match { let __cpv = c.compare_exchange(
            cur,
            cur + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c#942", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => break,
            Err(_) => continue, // failed attempt retried, never abandoned
        }
    }
}

fn w1(c: &Arc<AtomicUsize>) { worker(c); }
fn w2(c: &Arc<AtomicUsize>) { worker(c); }

fn main() { cir_trace::init();
    // R2: single shared counter starting at zero
    let c = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches w1 and w2, then joins both
    let handles = [
        {
            let c = Arc::clone(&c);
            cir_trace::spawn("w1#1116", move || w1(&c))
        },
        {
            let c = Arc::clone(&c);
            cir_trace::spawn("w2#1220", move || w2(&c))
        },
    ];

    for handle in &handles {
        handle.join().expect("worker panicked");
    }

    // R3/R6: both workers finished, so the counter must be 2 under
    // every schedule (each increment's CAS eventually succeeds).
    assert_eq!(c.load(Ordering::SeqCst), 2);

    // R9 as literally written ("DONE done=1") is unsatisfiable: R3/R6/R7
    // force the counter to 2, so the honest, consistent output reports it.
    let done = { let __cpv = c.load(Ordering::SeqCst); cir_trace::record_value("c#942", (__cpv) as i64); __cpv };
    println!("DONE done={}", done);
 cir_trace::finish();}
