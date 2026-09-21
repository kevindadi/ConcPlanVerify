use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let shared = Arc::new(Mutex::new(0));

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        // Communicate without holding the shared lock.
        tx.send(1).unwrap();

        // Then occasionally acquire the shared mutex.
        let mut guard = shared_s.lock().unwrap();
        *guard += 10;
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Receive while NOT holding the lock the sender may need.
        let v = rx.recv().unwrap();

        // Then use the shared mutex.
        let mut guard = shared_r.lock().unwrap();
        *guard += v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
