mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#350", 2);

    let s1 = s.clone();
    let s2 = s.clone();
    let s3 = s.clone();

    let h1 = crate::cir_trace::spawn("w1#445", move || w1(&s1));
    let h2 = crate::cir_trace::spawn("w2#490", move || w2(&s2));
    let h3 = crate::cir_trace::spawn("w3#535", move || w3(&s3));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    h3.join().expect("w3 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
