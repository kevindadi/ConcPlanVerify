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

    let h_w1 = std::thread::Builder::new().name("main::w1".to_string()).spawn({
        let s = s.clone();
        move || w1(s)
    }).unwrap();

    let h_w2 = std::thread::Builder::new().name("main::w2".to_string()).spawn({
        let s = s.clone();
        move || w2(s)
    }).unwrap();

    h_w1.join().unwrap();
    h_w2.join().unwrap();

    println!("DONE permits=1");
}
