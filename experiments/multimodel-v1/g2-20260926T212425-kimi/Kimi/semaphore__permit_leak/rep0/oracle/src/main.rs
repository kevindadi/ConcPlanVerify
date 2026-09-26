mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    // Acquire the single shared permit before working.
    let permit = s.acquire();

    // Perform work while holding the permit.
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);

    // Release the permit before finishing.
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    // Acquire the single shared permit before working.
    let permit = s.acquire();

    // Perform work while holding the permit.
    let mut acc: u64 = 1;
    for i in 1..10_000u64 {
        acc = acc.wrapping_mul(i % 97 + 1);
    }
    std::hint::black_box(acc);

    // Release the permit before finishing.
    permit.release();
}

fn main() { cir_trace::init();
    // One counting permit pool with exactly one permit, shared by both workers.
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1", move || w1(s_w1));
    let h2 = cir_trace::spawn("w2", move || w2(s_w2));

    // Supervising task waits for both workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
