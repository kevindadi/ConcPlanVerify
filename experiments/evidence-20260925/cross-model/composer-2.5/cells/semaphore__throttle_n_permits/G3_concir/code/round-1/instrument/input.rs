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

fn supervisor(s: Arc<Semaphore>) {
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
}

fn println() {
    std::println!("DONE done=1");
}

fn main() {
    let s = Semaphore::new(2);
    let h_sup = thread::spawn({
        let s = Arc::clone(&s);
        move || supervisor(s)
    });
    h_sup.join().unwrap();
    println();
}
