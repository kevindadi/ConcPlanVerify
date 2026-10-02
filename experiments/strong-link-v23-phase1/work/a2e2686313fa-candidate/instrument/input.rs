use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Channel with capacity 1: empty_slots and full_slots semaphores.
    let empty_slots = Semaphore::new(1);
    let full_slots = Semaphore::new(0);

    // Shared lock m.
    let m = Arc::new(Mutex::new(()));

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let empty_sender = Arc::clone(&empty_slots);
    let full_sender = Arc::clone(&full_slots);
    let empty_receiver = Arc::clone(&empty_slots);
    let full_receiver = Arc::clone(&full_slots);

    let sender = thread::spawn(move || {
        for i in 0..2 {
            // Acquire shared lock occasionally, but never while waiting on channel.
            {
                let _guard = m_sender.lock().unwrap();
                // critical section using shared lock
            }

            // Wait for an empty slot before sending.
            let permit = empty_sender.acquire();
            // send value i
            permit.release();
            // signal a full slot
            full_sender.acquire().release();
        }
    });

    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // Wait for a full slot before receiving.
            let permit = full_receiver.acquire();
            // receive value
            permit.release();
            // signal an empty slot
            empty_receiver.acquire().release();

            // Acquire shared lock occasionally, but never while waiting on channel.
            {
                let _guard = m_receiver.lock().unwrap();
                // critical section using shared lock
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
