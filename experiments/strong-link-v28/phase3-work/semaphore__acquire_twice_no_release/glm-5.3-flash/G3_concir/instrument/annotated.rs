mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work: i32 = 0;
    work = work + 1;
    let _ = work;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work: i32 = 0;
    work = work + 1;
    let _ = work;
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#397", 1);
    let s1 = s.clone();
    let s2 = s.clone();
    let h1 = cir_trace::spawn("w1#466", move || w1(s1));
    let h2 = cir_trace::spawn("w2#510", move || w2(s2));
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    println!("DONE done=1");
 cir_trace::finish();}
