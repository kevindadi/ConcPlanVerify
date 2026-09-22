mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, mpsc};
use std::thread;

fn main() { cir_trace::init();
    // ch1: rendezvous channel (0-capacity) used for the handshake.
    // ch2: channel used to report completion back to main.
    let (ch1_tx, ch1_rx) = mpsc::sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = mpsc::channel::<i32>();

    // Shared lock occasionally used by both roles.
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0i32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    // Sender role: s
    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Meet the receiver on ch1 (rendezvous). Not holding the lock here.
        ch1_tx.send(1).unwrap();

        // Use the lock again after the handshake.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Report completion.
        ch2_tx.send(1).unwrap();
    });

    // Receiver role: r
    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Meet the sender on ch1 (rendezvous). Not holding the lock here.
        let v = ch1_rx.recv().unwrap();

        // Use the lock again after the handshake.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Report completion.
        ch2_tx.send(v).unwrap();
    });

    // Wait for both roles to finish.
    let a = ch2_rx.recv().unwrap();
    let b = ch2_rx.recv().unwrap();

    s.join().unwrap();
    r.join().unwrap();

    let done = a + b;
    println!("DONE done={}", done);
 cir_trace::finish();}
