use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker(name: &str, s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit.
    println!("{} working", name);
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = thread::spawn(move || worker("w1", s1));
    let w2 = thread::spawn(move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
