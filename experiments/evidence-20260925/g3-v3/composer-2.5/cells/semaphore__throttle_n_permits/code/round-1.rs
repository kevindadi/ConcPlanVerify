use std::sync::{Arc, Semaphore};
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
    let mut done = 0;
    done = 1;
    let _ = done;
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
    let mut done = 0;
    done = 1;
    let _ = done;
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
    let mut done = 0;
    done = 1;
    let _ = done;
}

fn main() {
    let s = Arc::new(Semaphore::new(2));

    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w2(s2));

    let s3 = Arc::clone(&s);
    let h3 = thread::spawn(move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
