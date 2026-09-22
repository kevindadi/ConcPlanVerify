mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, mpsc};
use std::thread;

fn main() { cir_trace::init();
    // ch1: rendezvous channel (0-capacity) used for the handshake.
    // ch2: channel used to report completion back to main.
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::channel::<i32>();

    // Shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0i32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    // Sender role.
    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Meet the receiver over ch1 (rendezvous).
        tx1.send(1).unwrap();

        // Use the shared lock again after the channel meeting.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        tx2.send(1).unwrap();
    });

    // Receiver role.
    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Meet the sender over ch1 (rendezvous).
        let v = rx1.recv().unwrap();

        // Use the shared lock again after the channel meeting.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        tx2.send(v).unwrap();
    });

    // Wait for both roles to finish.
    let a = rx2.recv().unwrap();
    let b = rx2.recv().unwrap();

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done={}", a + b);
 cir_trace::finish();}
