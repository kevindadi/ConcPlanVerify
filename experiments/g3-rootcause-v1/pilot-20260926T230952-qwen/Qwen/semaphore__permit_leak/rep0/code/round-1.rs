use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let h1 = std::thread::spawn({
        let s = s.clone();
        move || w1(s)
    });

    let h2 = std::thread::spawn({
        let s = s.clone();
        move || w2(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE permits=1");
}
