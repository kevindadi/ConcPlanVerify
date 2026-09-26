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
    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let s_w3 = Arc::clone(&s);

    let h1 = thread::spawn(move || w1(s_w1));
    let h2 = thread::spawn(move || w2(s_w2));
    let h3 = thread::spawn(move || w3(s_w3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
