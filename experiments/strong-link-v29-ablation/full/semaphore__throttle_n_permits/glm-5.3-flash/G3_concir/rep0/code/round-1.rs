use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    let _ = permit;
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    let _ = permit;
    permit.release();
}

fn w3(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    let _ = permit;
    permit.release();
}

fn main() {
    let s = Semaphore::new(2);

    let h1 = thread::spawn(move || w1(&s));
    let s2 = s.clone();
    let h2 = thread::spawn(move || w2(&s2));
    let s3 = s.clone();
    let h3 = thread::spawn(move || w3(&s3));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    h3.join().expect("w3 panicked");

    println!("DONE done=1");
}
