mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    // s1: acquire
    let permit = s.acquire();
    // s2: release
    permit.release();
    // s3: acquire
    let permit = s.acquire();
    // s4: release
    permit.release();
    // s5: return
}

fn w2(s: &Arc<Semaphore>) {
    // s1: acquire
    let permit = s.acquire();
    // s2: release
    permit.release();
    // s3: acquire
    let permit = s.acquire();
    // s4: release
    permit.release();
    // s5: return
}

fn main() { crate::cir_trace::init();
    // Resource s: counting semaphore with exactly one initial permit.
    let s = Semaphore::new_named("s_semaphore0#630", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = crate::cir_trace::spawn("w1#711", move || w1(&s1));
    let h2 = crate::cir_trace::spawn("w2#756", move || w2(&s2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
