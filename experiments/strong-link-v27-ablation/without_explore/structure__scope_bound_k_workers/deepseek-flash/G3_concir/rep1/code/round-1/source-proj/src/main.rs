use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut acc: i32 = 0;
    let permit = s.acquire();
    acc = 1;
    permit.release();
    let _ = acc;
}

fn w2(s: Arc<Semaphore>) {
    let mut acc: i32 = 0;
    let permit = s.acquire();
    acc = 1;
    permit.release();
    let _ = acc;
}

fn w3(s: Arc<Semaphore>) {
    let mut acc: i32 = 0;
    let permit = s.acquire();
    acc = 1;
    permit.release();
    let _ = acc;
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let s2 = s.clone();
    let s3 = s.clone();
    let s4 = s.clone();
    let s5 = s.clone();
    let s6 = s.clone();

    let h1 = thread::spawn(move || w1(s1));
    let h2 = thread::spawn(move || w1(s2));
    let h3 = thread::spawn(move || w2(s3));
    let h4 = thread::spawn(move || w2(s4));
    let h5 = thread::spawn(move || w3(s5));
    let h6 = thread::spawn(move || w3(s6));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();
    h5.join().unwrap();
    h6.join().unwrap();

    println!("DONE done=1");
}
