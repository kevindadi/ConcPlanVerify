use std::sync::{Arc, Semaphore};
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
    let mut work = 0;
    work = 1;
    drop(_permit);

    let _permit = s.acquire().unwrap();
    work = 2;
    drop(_permit);
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
    let mut work = 0;
    work = 1;
    drop(_permit);

    let _permit = s.acquire().unwrap();
    work = 2;
    drop(_permit);
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    let s_w1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s_w1));

    let s_w2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w2(s_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
