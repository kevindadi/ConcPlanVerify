mod cir_trace;
use concir_sync::Semaphore;
use std::thread;
use std::sync::Arc;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    // w1 performs its work while holding the permit
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    // w2 performs its work while holding the permit
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s: Arc<Semaphore> = Semaphore::new_named("s_semaphore0#389", 1);

    let s1 = Arc::clone(&s);
    let t1 = crate::cir_trace::spawn("w1#440", move || {
        w1(&s1);
    });

    let s2 = Arc::clone(&s);
    let t2 = crate::cir_trace::spawn("w2#532", move || {
        w2(&s2);
    });

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    println!("DONE permits=1");
 crate::cir_trace::finish();}
