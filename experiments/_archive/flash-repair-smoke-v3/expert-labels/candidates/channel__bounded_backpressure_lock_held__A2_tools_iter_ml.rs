use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));

    let m1 = Arc::clone(&m);
    let s = thread::spawn(move || {
        // Send first value without holding the lock.
        tx.send(1).unwrap();
        // Briefly take the lock, then release before the second send.
        {
            let _g = m1.lock().unwrap();
        }
        tx.send(2).unwrap();
    });

    let r = thread::spawn(move || {
        // Receive first value without holding the lock.
        rx.recv().unwrap();
        // Briefly take the lock, then release before the second receive.
        {
            let _g = m.lock().unwrap();
        }
        rx.recv().unwrap();
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
