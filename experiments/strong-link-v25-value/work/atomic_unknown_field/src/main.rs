mod cir_trace;
// UNKNOWN control: the counter is a struct field reached through a field
// receiver. The simple path-receiver rule does not observe it, so the value is
// unknown (unsupported), never guessed from the printed text.
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

struct S { c: AtomicI32 }

fn w1(s: Arc<S>) {
    loop {
        let observed = s.c.load(Ordering::SeqCst);
        let old = s.c.compare_exchange(observed, observed + 1, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or_else(|old| old);
        if old == observed { return; }
    }
}

fn main() { crate::cir_trace::init();
    let s = Arc::new(S { c: AtomicI32::new(0) });
    let s1 = Arc::clone(&s);
    let h = crate::cir_trace::spawn("w1#699", move || w1(s1));
    h.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
