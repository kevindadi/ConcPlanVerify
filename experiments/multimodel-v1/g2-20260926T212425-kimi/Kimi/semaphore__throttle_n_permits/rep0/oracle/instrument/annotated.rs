mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;
use std::hint::black_box;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding one permit.
    black_box(1u64.wrapping_mul(3));
    drop(permit); // release the permit so a waiting worker can proceed
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding one permit.
    black_box(2u64.wrapping_mul(3));
    drop(permit); // release the permit so a waiting worker can proceed
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding one permit.
    black_box(3u64.wrapping_mul(3));
    drop(permit); // release the permit so a waiting worker can proceed
}

fn main() { cir_trace::init();
    // Supervising task: one shared pool with exactly two permits.
    let s = Semaphore::new_named("s_semaphore0", 2);

    let h1 = cir_trace::spawn("h1", {
        let s = s.clone();
        move || w1(s)
    });
    let h2 = cir_trace::spawn("h2", {
        let s = s.clone();
        move || w2(s)
    });
    let h3 = cir_trace::spawn("h3", {
        let s = s.clone();
        move || w3(s)
    });

    // Wait for all three workers to finish.
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
