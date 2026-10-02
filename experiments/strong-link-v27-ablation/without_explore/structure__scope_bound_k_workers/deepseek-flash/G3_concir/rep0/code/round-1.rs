use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

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
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s1));
    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w1(s2));
    let s3 = Arc::clone(&s);
    let h3 = thread::spawn(move || w2(s3));
    let s4 = Arc::clone(&s);
    let h4 = thread::spawn(move || w2(s4));
    let s5 = Arc::clone(&s);
    let h5 = thread::spawn(move || w3(s5));
    let s6 = Arc::clone(&s);
    let h6 = thread::spawn(move || w3(s6));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();
    h5.join().unwrap();
    h6.join().unwrap();

    println!("DONE done=1");
}
