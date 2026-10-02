mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn w2(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn w3(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#476", 1);

    let s1 = Arc::clone(&s);
    let t1 = crate::cir_trace::spawn("w1#527", move || w1(&s1));
    let s2 = Arc::clone(&s);
    let t2 = crate::cir_trace::spawn("w2#601", move || w2(&s2));
    let s3 = Arc::clone(&s);
    let t3 = crate::cir_trace::spawn("w3#675", move || w3(&s3));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");
    t3.join().expect("w3 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
