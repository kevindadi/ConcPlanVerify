mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    // work while holding the permit
    permit.release();
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    // work while holding the permit
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#311", 1);

    let s1 = s.clone();
    let h1 = crate::cir_trace::spawn("w1#357", move || w1(&s1));

    let s2 = s.clone();
    let h2 = crate::cir_trace::spawn("w2#427", move || w2(&s2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE permits=1");
 crate::cir_trace::finish();}
