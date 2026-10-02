mod cir_trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn worker(c: Arc<AtomicUsize>, name: &'static str) {
    loop {
        let cur = { let __cpv = c.load(Ordering::Relaxed); cir_trace::record_value("c", (__cpv) as i64); __cpv };
        // compare_exchange provides an indivisible read-modify-write (R7).
        // On failure (a competing update won the race) we simply retry (R4, R5);
        // the value can only ever move forward toward the final total (R6),
        // and retrying always makes progress, so termination is guaranteed (R8).
        match { let __cpv = c.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ); if __cpv.is_ok() { cir_trace::record_value("c", (c.load(std::sync::atomic::Ordering::SeqCst)) as i64); } __cpv } {
            Ok(_) => {
                // Successfully contributed exactly one increment (R3).
                break;
            }
            Err(_) => {
                // Lost the race — retry rather than abandon (R5).
                std::hint::spin_loop();
            }
        }
    }
    let _ = name; // w1 / w2 roles; kept for traceability.
}

fn main() { cir_trace::init();
    // R1: supervising task launches two workers and waits for both.
    let c = Arc::new(AtomicUsize::new(0)); // R2: starts at zero.
    let h1 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("worker#1203", move || worker(c, "w1"))
    };
    let h2 = {
        let c = Arc::clone(&c);
        cir_trace::spawn("worker#1309", move || worker(c, "w2"))
    };
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    // R9: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
