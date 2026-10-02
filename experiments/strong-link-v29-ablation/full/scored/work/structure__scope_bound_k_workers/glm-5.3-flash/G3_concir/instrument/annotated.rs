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
    let s = Semaphore::new_named("s_semaphore0#350", 1);

    let s1 = s.clone();
    let h1 = crate::cir_trace::spawn("w1#396", move || w1(&s1));
    let s2 = s.clone();
    let h2 = crate::cir_trace::spawn("w2#465", move || w2(&s2));
    let s3 = s.clone();
    let h3 = crate::cir_trace::spawn("w3#534", move || w3(&s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
