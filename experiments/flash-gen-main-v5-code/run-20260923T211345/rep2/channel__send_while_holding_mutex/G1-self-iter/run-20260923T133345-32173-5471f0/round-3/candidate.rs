use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // ch1: rendezvous channel implemented with a semaphore pair.
    // Each semaphore starts with one permit so the signaling side can
    // acquire-then-release to hand the permit to the waiting side.
    let ch1_send = Semaphore::new(1);
    let ch1_recv = Semaphore::new(1);

    // ch2: completion signal channel.
    let ch2 = Semaphore::new(1);

    // Shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new(()));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_send_s = Arc::clone(&ch1_send);
    let ch1_recv_s = Arc::clone(&ch1_recv);
    let ch2_s = Arc::clone(&ch2);

    let ch1_send_r = Arc::clone(&ch1_send);
    let ch1_recv_r = Arc::clone(&ch1_recv);
    let ch2_r = Arc::clone(&ch2);

    // Sender role s
    let s = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let _g = lock_s.lock().unwrap();
            // critical section
        }

        // Rendezvous on ch1: signal readiness, then wait for receiver to take it.
        let p = ch1_send_s.acquire(); // take our readiness permit
        p.release();                  // hand it to the receiver
        ch1_recv_s.acquire();         // wait for receiver to take the value

        // Signal completion on ch2.
        let p = ch2_s.acquire();
        p.release();
    });

    // Receiver role r
    let r = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let _g = lock_r.lock().unwrap();
            // critical section
        }

        // Rendezvous on ch1: wait for sender to be ready, then take the value.
        ch1_send_r.acquire();         // wait for sender's readiness
        let p = ch1_recv_r.acquire(); // take our ack permit
        p.release();                  // signal sender we took it

        // Wait for completion signal on ch2.
        ch2_r.acquire();
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
