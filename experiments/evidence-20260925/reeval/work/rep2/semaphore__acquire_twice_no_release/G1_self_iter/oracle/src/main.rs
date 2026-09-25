mod cir_trace;
use std::thread;
use std::sync::Arc;
use concir_sync::Semaphore;

fn worker(name: &str, s: Arc<Semaphore>) {
    // Acquire the permit, do work, release it.
    let permit = s.acquire();
    println!("{} working", name);
    // Simulate work
    thread::yield_now();
    permit.release();

    // Acquire again to demonstrate multiple acquisitions.
    let permit2 = s.acquire();
    println!("{} working again", name);
    thread::yield_now();
    permit2.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = cir_trace::spawn("worker", move || worker("w1", s1));
    let w2 = cir_trace::spawn("worker", move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
