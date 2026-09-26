use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let permit = s.acquire();
    done.store(1, Ordering::SeqCst);
    permit.release();
}

fn w2(s: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let permit = s.acquire();
    done.store(1, Ordering::SeqCst);
    permit.release();
}

fn w3(s: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    let permit = s.acquire();
    done.store(1, Ordering::SeqCst);
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);
    let done = Arc::new(AtomicUsize::new(0));

    let h1a = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w1(s, done)
    });
    let h1b = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w1(s, done)
    });
    let h2a = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w2(s, done)
    });
    let h2b = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w2(s, done)
    });
    let h3a = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w3(s, done)
    });
    let h3b = thread::spawn({
        let s = Arc::clone(&s);
        let done = Arc::clone(&done);
        move || w3(s, done)
    });

    h1a.join().unwrap();
    h1b.join().unwrap();
    h2a.join().unwrap();
    h2b.join().unwrap();
    h3a.join().unwrap();
    h3b.join().unwrap();

    println!("DONE done={}", done.load(Ordering::SeqCst));
}
