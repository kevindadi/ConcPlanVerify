use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut i = 0;
    while i < 2 {
        let permit = s.acquire();
        permit.release();
        i += 1;
    }
}

fn w2(s: Arc<Semaphore>) {
    let mut i = 0;
    while i < 2 {
        let permit = s.acquire();
        permit.release();
        i += 1;
    }
}

fn w3(s: Arc<Semaphore>) {
    let mut i = 0;
    while i < 2 {
        let permit = s.acquire();
        permit.release();
        i += 1;
    }
}

fn main() {
    let s = Semaphore::new(1);

    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(s))
    };
    let h3 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
}
