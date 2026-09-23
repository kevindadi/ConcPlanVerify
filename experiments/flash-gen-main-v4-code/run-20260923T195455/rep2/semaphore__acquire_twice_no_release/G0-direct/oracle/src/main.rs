mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker(name: &'static str, s: Arc<Semaphore>) {
    // Acquire the permit, do work, then acquire again and release both.
    let permit1 = s.acquire();
    // critical work while holding permit1
    let permit2 = s.acquire();
    // more work while holding both permits
    drop(permit2);
    drop(permit1);
    let _ = name;
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
