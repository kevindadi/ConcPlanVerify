use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // ch1: rendezvous channel (capacity 0) implemented with a semaphore pair.
    // We use two semaphores to implement a synchronous handoff.
    let ch1_send = Semaphore::new(0);
    let ch1_recv = Semaphore::new(0);

    // ch2: another channel used to signal completion.
    let ch2_send = Semaphore::new(0);
    let ch2_recv = Semaphore::new(0);

    // Shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new(()));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_send_s = Arc::clone(&ch1_send);
    let ch1_recv_s = Arc::clone(&ch1_recv);
    let ch2_send_s = Arc::clone(&ch2_send);

    let ch1_send_r = Arc::clone(&ch1_send);
    let ch1_recv_r = Arc::clone(&ch1_recv);
    let ch2_recv_r = Arc::clone(&ch2_recv);

    // Sender role s
    let s = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let _g = lock_s.lock().unwrap();
            // critical section
        }

        // Rendezvous on ch1: signal we are ready, then wait for receiver.
        ch1_send_s.acquire(); // wait until receiver is ready
        // handoff value (conceptually)
        ch1_recv_s.acquire(); // wait for receiver to take it

        // Signal completion on ch2.
        ch2_send_s.acquire();
    });

    // Receiver role r
    let r = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let _g = lock_r.lock().unwrap();
            // critical section
        }

        // Rendezvous on ch1: signal we are ready, then take the value.
        ch1_send_r.acquire(); // wait for sender to be ready
        // take value (conceptually)
        ch1_recv_r.acquire(); // signal sender we took it

        // Wait for completion signal on ch2.
        ch2_recv_r.acquire();
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
