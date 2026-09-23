use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new(()));

    // ch1: rendezvous channel (requires both roles to meet).
    // Implemented as a zero-capacity rendezvous using two semaphores.
    let ch1_send = Semaphore::new(0);
    let ch1_recv = Semaphore::new(0);

    // ch2: another channel used to exchange the final value.
    let ch2_send = Semaphore::new(0);
    let ch2_recv = Semaphore::new(0);

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_send_s = Arc::clone(&ch1_send);
    let ch1_recv_s = Arc::clone(&ch1_recv);
    let ch2_send_s = Arc::clone(&ch2_send);
    let ch2_recv_s = Arc::clone(&ch2_recv);

    let ch1_send_r = Arc::clone(&ch1_send);
    let ch1_recv_r = Arc::clone(&ch1_recv);
    let ch2_send_r = Arc::clone(&ch2_send);
    let ch2_recv_r = Arc::clone(&ch2_recv);

    // Sender role s.
    let s = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let _g = lock_s.lock().unwrap();
        }

        // Rendezvous on ch1: signal we are ready, then wait for receiver.
        ch1_send_s.acquire();
        ch1_recv_s.acquire();

        // Exchange value over ch2.
        ch2_send_s.acquire();
        ch2_recv_s.acquire();

        // Use the shared lock again, not while waiting on a channel.
        {
            let _g = lock_s.lock().unwrap();
        }
    });

    // Receiver role r.
    let r = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let _g = lock_r.lock().unwrap();
        }

        // Rendezvous on ch1: wait for sender, then signal we met.
        ch1_recv_r.acquire();
        ch1_send_r.acquire();

        // Exchange value over ch2.
        ch2_recv_r.acquire();
        ch2_send_r.acquire();

        // Use the shared lock again, not while waiting on a channel.
        {
            let _g = lock_r.lock().unwrap();
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
