mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#360", 1);

    let s1 = s.clone();
    let t1 = crate::cir_trace::spawn("w1#406", move || w1(s1));

    let s2 = s.clone();
    let t2 = crate::cir_trace::spawn("w2#475", move || w2(s2));

    let s3 = s.clone();
    let t3 = crate::cir_trace::spawn("w3#544", move || w3(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
