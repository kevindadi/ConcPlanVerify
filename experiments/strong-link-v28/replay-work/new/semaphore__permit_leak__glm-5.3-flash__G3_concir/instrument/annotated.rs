mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn main() { cir_trace::init();
    let s: Arc<Semaphore> = Semaphore::new_named("s_semaphore0#100", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#151", move || {
        w1(&s1);
    });

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w2#248", move || {
        w2(&s2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE permits=1");
 cir_trace::finish();}

fn w1(s: &Semaphore) {
    let _permit = s.acquire();
    let work: i64 = 1;
    let _ = work;
    _permit.release();
}

fn w2(s: &Semaphore) {
    let _permit = s.acquire();
    let work: i64 = 1;
    let _ = work;
    _permit.release();
}
