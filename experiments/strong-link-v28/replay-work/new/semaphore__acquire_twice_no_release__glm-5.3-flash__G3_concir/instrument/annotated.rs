mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    // work happens while holding the permit
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    // work happens while holding the permit
    permit.release();
}

fn main() { cir_trace::init();
    let s: Arc<Semaphore> = Semaphore::new_named("s_semaphore0#373", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#424", move || {
        w1(&s1);
    });

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w2#516", move || {
        w2(&s2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
