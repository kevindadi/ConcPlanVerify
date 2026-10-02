use concir_sync::Semaphore;
use std::thread;

fn worker(name: &str, s: std::sync::Arc<Semaphore>) {
    // Acquire the permit, do work, release it.
    let permit = s.acquire();
    println!("{} working", name);
    permit.release();

    // Acquire again to demonstrate multiple acquire/release cycles.
    let permit2 = s.acquire();
    println!("{} working again", name);
    permit2.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let s2 = s.clone();

    let w1 = thread::spawn(move || worker("w1", s1));
    let w2 = thread::spawn(move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
