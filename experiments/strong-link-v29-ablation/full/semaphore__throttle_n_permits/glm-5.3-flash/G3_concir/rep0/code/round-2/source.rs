use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(2);

    let s1 = s.clone();
    let s2 = s.clone();
    let s3 = s.clone();

    let h1 = thread::spawn(move || w1(&s1));
    let h2 = thread::spawn(move || w2(&s2));
    let h3 = thread::spawn(move || w3(&s3));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    h3.join().expect("w3 panicked");

    println!("DONE done=1");
}
