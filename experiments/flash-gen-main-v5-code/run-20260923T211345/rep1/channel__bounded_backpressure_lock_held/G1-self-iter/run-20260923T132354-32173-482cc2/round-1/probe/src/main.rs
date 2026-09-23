use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Channel with capacity 1: use two semaphores to model empty/full slots.
    // `empty` starts with 1 permit (one free slot), `full` starts with 0 permits.
    let empty = Semaphore::new(1);
    let full = Semaphore::new(0);

    // Shared lock `m`.
    let m = Arc::new(Mutex::new(()));

    let empty_s = empty.clone();
    let full_s = full.clone();
    let m_s = m.clone();

    let sender = thread::spawn(move || {
        // First value
        {
            let _guard = m_s.lock().unwrap();
            // do some work with the shared lock, but do not wait on channel here
        }
        // Wait for a free slot before sending (not holding the lock).
        let permit = empty_s.acquire();
        // send value 1
        permit.release();
        full_s.acquire().release(); // signal full

        // Second value
        {
            let _guard = m_s.lock().unwrap();
        }
        let permit = empty_s.acquire();
        // send value 2
        permit.release();
        full_s.acquire().release();
    });

    let empty_r = empty.clone();
    let full_r = full.clone();
    let m_r = m.clone();

    let receiver = thread::spawn(move || {
        // Receive first value
        full_r.acquire().release(); // wait until full
        {
            let _guard = m_r.lock().unwrap();
        }
        empty_r.acquire().release(); // signal empty

        // Receive second value
        full_r.acquire().release();
        {
            let _guard = m_r.lock().unwrap();
        }
        empty_r.acquire().release();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
