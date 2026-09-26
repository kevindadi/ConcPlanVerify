use concir_sync::Semaphore;
use std::thread;

fn w1(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn w2(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn w3(s: std::sync::Arc<Semaphore>) {
    let permit_kept = s.acquire();
    permit_kept.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let s2 = s.clone();
    let s3 = s.clone();

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w2(s2));
    let h3 = thread::spawn(move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
