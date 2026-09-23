use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    drop(permit);
}

fn main() {
    let s = Semaphore::new(1);
    let mut done = 0;

    let h1 = {
        let s = s.clone();
        thread::spawn(move || w1(s))
    };
    let h2 = {
        let s = s.clone();
        thread::spawn(move || w2(s))
    };
    let h3 = {
        let s = s.clone();
        thread::spawn(move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
