mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker(name: &'static str, s: Arc<Semaphore>) {
    // Acquire the permit once.
    let permit1 = s.acquire();
    // Do some work while holding the permit.
    println!("{} working (1)", name);
    // Acquire the permit a second time (nested acquisition).
    let permit2 = s.acquire();
    println!("{} working (2)", name);
    // Release the second permit.
    permit2.release();
    // Release the first permit.
    permit1.release();
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
