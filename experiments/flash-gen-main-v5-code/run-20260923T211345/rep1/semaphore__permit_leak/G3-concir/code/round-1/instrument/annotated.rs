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

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = s.clone();
    let h1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = s.clone();
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
