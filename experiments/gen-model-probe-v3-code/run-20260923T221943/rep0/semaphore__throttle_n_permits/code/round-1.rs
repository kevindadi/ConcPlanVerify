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
    let s: Arc<Semaphore> = Semaphore::new(2);
    let mut done = 0;

    let h1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let h2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h3 = thread::spawn({
        let s = Arc::clone(&s);
        move || w3(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
