mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let p1 = s.acquire();
    p1.release();
    let p2 = s.acquire();
    p2.release();
}

fn w2(s: &Semaphore) {
    let p1 = s.acquire();
    p1.release();
    let p2 = s.acquire();
    p2.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#309", 1);
    let s1 = std::sync::Arc::clone(&s);
    let h1 = crate::cir_trace::spawn("w1#370", move || w1(&s1));
    let s2 = std::sync::Arc::clone(&s);
    let h2 = crate::cir_trace::spawn("w2#455", move || w2(&s2));
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
