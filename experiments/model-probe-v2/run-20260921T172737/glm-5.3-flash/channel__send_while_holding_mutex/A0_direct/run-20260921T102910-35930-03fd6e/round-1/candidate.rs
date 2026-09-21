use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let lock = Arc::new(Mutex::new(0i32));

    let lock_s = Arc::clone(&lock);
    let sender = thread::spawn(move || {
        // Take the shared mutex only while NOT blocked on the channel:
        // send happens after the lock guard is dropped.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
        tx.send(1).unwrap();
    });

    let lock_r = Arc::clone(&lock);
    let receiver = thread::spawn(move || {
        // Receive first (not holding the lock), then touch the mutex.
        let v = rx.recv().unwrap();
        let mut g = lock_r.lock().unwrap();
        *g += v;
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    let done = *lock.lock().unwrap();
    assert_eq!(done, 2);
    println!("DONE done=1");
}
