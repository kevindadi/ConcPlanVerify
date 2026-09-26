mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn w2(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn w3(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let s1 = s.clone();
    let s2 = s.clone();
    let s3 = s.clone();

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));
    let h3 = cir_trace::spawn("w3", move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
