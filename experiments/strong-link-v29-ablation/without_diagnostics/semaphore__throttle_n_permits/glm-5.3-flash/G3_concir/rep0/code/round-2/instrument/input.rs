use std::sync::Arc;
use std::thread;

use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // worker performs its work while holding one permit
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // worker performs its work while holding one permit
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // worker performs its work while holding one permit
    permit.release();
}

fn main() {
    let s = Semaphore::new(2);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));
    let h3 = thread::spawn(move || w3(s3));
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
}
