use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // ch1: rendezvous channel (capacity 0) implemented with two semaphores.
    // A sender waits for the receiver to be ready, then signals completion.
    let ch1_ready = Semaphore::new(0);   // receiver signals readiness
    let ch1_done = Semaphore::new(0);    // sender signals value delivered

    // ch2: another rendezvous channel used to confirm completion.
    let ch2_ready = Semaphore::new(0);
    let ch2_done = Semaphore::new(0);

    // Shared lock occasionally used by both roles.
    let lock = Arc::new(Mutex::new(0u32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s_ready = Arc::clone(&ch1_ready);
    let s_done = Arc::clone(&ch1_done);
    let s2_ready = Arc::clone(&ch2_ready);
    let s2_done = Arc::clone(&ch2_done);

    let r_ready = Arc::clone(&ch1_ready);
    let r_done = Arc::clone(&ch1_done);
    let r2_ready = Arc::clone(&ch2_ready);
    let r2_done = Arc::clone(&ch2_done);

    let sender = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: wait for receiver, then deliver.
        let permit = s_ready.acquire();
        permit.release();
        s_done.acquire().release();

        // Rendezvous on ch2 to confirm.
        let permit2 = s2_ready.acquire();
        permit2.release();
        s2_done.acquire().release();
    });

    let receiver = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: signal readiness, then wait for delivery.
        let permit = r_ready.acquire();
        permit.release();
        r_done.acquire().release();

        // Rendezvous on ch2 to confirm.
        let permit2 = r2_ready.acquire();
        permit2.release();
        r2_done.acquire().release();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
