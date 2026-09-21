use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    // Channel of capacity one.
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    // A mutex both sides occasionally need.
    let m = Arc::new(Mutex::new(0usize));

    let m1 = Arc::clone(&m);
    let sender = thread::spawn(move || {
        // Use the mutex, but release it before any channel operation so we
        // never block on the channel while holding the lock the receiver needs.
        {
            let mut g = m1.lock().unwrap();
            *g += 1;
        } // guard dropped here

        // First value fits in the capacity-one buffer.
        tx.send(1).unwrap();

        // Touch the mutex again, still without holding it across a send.
        {
            let mut g = m1.lock().unwrap();
            *g += 1;
        }

        // Second send may block until the receiver takes the first value;
        // we hold no lock here, so the receiver is free to proceed.
        tx.send(2).unwrap();
    });

    let m2 = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        // Receive both values without holding the mutex, so the sender is
        // never blocked on the lock while we wait on the channel.
        let a = rx.recv().unwrap();
        let b = rx.recv().unwrap();

        // Only after all channel work is done do we take the mutex.
        let mut g = m2.lock().unwrap();
        *g += a + b;
    });

    sender.join().unwrap();
    receiver.join().unwrap();
    println!("DONE done=1");
}
