use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn main() {
    // ch1: rendezvous channel (sync_channel(0)) used for the exchange.
    // ch2: channel used to report the received value back to main.
    let (tx1, rx1): (Sender<i32>, Receiver<i32>) = channel();
    let (tx2, rx2): (Sender<i32>, Receiver<i32>) = channel();

    // Shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new(0));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    // Sender role s
    let s = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Send the value over ch1 (rendezvous).
        tx1.send(1).unwrap();

        // Use the lock again after the exchange.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
    });

    // Receiver role r
    let r = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Receive the value over ch1 (rendezvous).
        let v = rx1.recv().unwrap();

        // Use the lock again after the exchange.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Report the received value back to main.
        tx2.send(v).unwrap();
    });

    let done = rx2.recv().unwrap();

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done={}", done);
}
