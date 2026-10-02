use concir_sync::Semaphore;
use std::thread;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(2);

    let h1 = thread::spawn(move || w1(s.clone()));
    let h2 = thread::spawn(move || w2(s.clone()));
    let h3 = thread::spawn(move || w3(s.clone()));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
