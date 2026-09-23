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

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = thread::spawn(move || worker("w1", s1));
    let w2 = thread::spawn(move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
