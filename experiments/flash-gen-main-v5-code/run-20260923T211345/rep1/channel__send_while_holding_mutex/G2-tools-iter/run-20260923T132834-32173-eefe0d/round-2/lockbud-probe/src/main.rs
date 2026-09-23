use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // ch1: rendezvous channel (capacity 0) implemented with two semaphores.
    // Receiver signals readiness; sender waits, then signals delivery.
    let ch1_ready = Semaphore::new(0);   // receiver signals readiness
    let ch1_done = Semaphore::new(0);    // sender signals value delivered

    // ch2: another rendezvous channel used to confirm completion.
    // Sender signals readiness; receiver waits, then signals confirmation.
    let ch2_ready = Semaphore::new(0);
    let ch2_done = Semaphore::new(0);

    // Shared lock occasionally used by both roles.
    let lock = Arc::new(Mutex::new(0u32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s1_ready = Arc::clone(&ch1_ready);
    let s1_done = Arc::clone(&ch1_done);
    let s2_ready = Arc::clone(&ch2_ready);
    let s2_done = Arc::clone(&ch2_done);

    let r1_ready = Arc::clone(&ch1_ready);
    let r1_done = Arc::clone(&ch1_done);
    let r2_ready = Arc::clone(&ch2_ready);
    let r2_done = Arc::clone(&ch2_done);

    let sender = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: wait for receiver readiness, then deliver.
        s1_ready.acquire().release();
        s1_done.acquire().release();

        // Rendezvous on ch2: signal readiness, then wait for confirmation.
        s2_ready.acquire().release();
        s2_done.acquire().release();
    });

    let receiver = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on a channel.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: signal readiness, then wait for delivery.
        r1_ready.acquire().release();
        r1_done.acquire().release();

        // Rendezvous on ch2: wait for sender readiness, then confirm.
        r2_ready.acquire().release();
        r2_done.acquire().release();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
