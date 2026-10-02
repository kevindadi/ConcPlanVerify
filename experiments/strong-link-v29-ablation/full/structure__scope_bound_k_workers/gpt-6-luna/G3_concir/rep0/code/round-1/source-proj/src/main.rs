use concir_sync::Semaphore;
use std::thread;

fn w1(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: std::sync::Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let t1 = thread::spawn(move || w1(s1));

    let s2 = s.clone();
    let t2 = thread::spawn(move || w2(s2));

    let s3 = s.clone();
    let t3 = thread::spawn(move || w3(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
