use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, enter: Arc<Semaphore>) {
    let _enter = enter.acquire();
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, enter: Arc<Semaphore>) {
    let _enter = enter.acquire();
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, enter: Arc<Semaphore>) {
    thread::scope(|s| {
        s.spawn({
            let a = a.clone();
            let b = b.clone();
            let enter = enter.clone();
            move || x1(a, b, enter)
        });
        s.spawn({
            let a = a.clone();
            let b = b.clone();
            let enter = enter.clone();
            move || x2(a, b, enter)
        });
    });
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let enter = Semaphore::new(1);
    let done = AtomicUsize::new(0);

    thread::scope(|s| {
        s.spawn({
            let a = a.clone();
            let b = b.clone();
            let enter = enter.clone();
            move || outer(a, b, enter)
        });
    });

    done.store(1, Ordering::SeqCst);
    println!("DONE done={}", done.load(Ordering::SeqCst));
}
