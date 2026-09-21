use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<i32>();
    let shared = Arc::new(Mutex::new(0));
    let shared_s = Arc::clone(&shared);
    let shared_r = Arc::clone(&shared);

    let s = thread::spawn(move || {
        // Communicate first, then acquire the shared lock.
        tx.send(1).unwrap();
        let mut guard = shared_s.lock().unwrap();
        *guard = 1;
    });

    let r = thread::spawn(move || {
        // Receive without holding the lock, then acquire it.
        let msg = rx.recv().unwrap();
        let mut guard = shared_r.lock().unwrap();
        *guard = msg;
    });

    s.join().unwrap();
    r.join().unwrap();

    let done = *shared.lock().unwrap();
    println!("DONE done={}", done);
}
