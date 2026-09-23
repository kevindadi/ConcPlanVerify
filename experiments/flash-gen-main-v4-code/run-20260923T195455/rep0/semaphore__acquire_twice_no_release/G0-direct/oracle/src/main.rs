mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker(name: &str, s: Arc<Semaphore>) {
    // Acquire the permit for the first unit of work.
    let permit1 = s.acquire();

    // Perform work while holding the permit.
    // The worker may acquire the permit more than once.
    let permit2 = s.acquire();

    // Release exactly as many times as acquired, in reverse order.
    drop(permit2);
    drop(permit1);

    // Ensure the name is used so the compiler does not warn.
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
