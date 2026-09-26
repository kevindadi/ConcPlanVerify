use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire();
}

fn supervisor(s: Arc<Semaphore>) {
    let h1 = std::thread::spawn({
        let s = s.clone();
        move || w1(s)
    });
    let h2 = std::thread::spawn({
        let s = s.clone();
        move || w2(s)
    });
    let h3 = std::thread::spawn({
        let s = s.clone();
        move || w3(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
}

fn main() {
    let s = Semaphore::new(2);
    let h_sup = std::thread::spawn({
        let s = s.clone();
        move || supervisor(s)
    });
    h_sup.join().unwrap();
    println!("DONE done=1");
}
