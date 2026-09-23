use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Channel with capacity 1: a slot guarded by a mutex, plus two semaphores
    // to track emptiness/fullness.
    let slot: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
    let empty = Semaphore::new(1); // initially empty
    let full = Semaphore::new(0);  // initially not full

    // Shared lock m.
    let m = Arc::new(Mutex::new(()));

    let slot_s = Arc::clone(&slot);
    let empty_s = Arc::clone(&empty);
    let full_s = Arc::clone(&full);
    let m_s = Arc::clone(&m);

    let sender = thread::spawn(move || {
        for v in [1, 2] {
            // Acquire shared lock m, but do not hold it while waiting on channel.
            {
                let _g = m_s.lock().unwrap();
            }

            // Wait until channel is empty, then put value.
            let permit = empty_s.acquire();
            {
                let mut s = slot_s.lock().unwrap();
                *s = Some(v);
            }
            permit.release();
            full_s.acquire().release();
        }
    });

    let slot_r = Arc::clone(&slot);
    let empty_r = Arc::clone(&empty);
    let full_r = Arc::clone(&full);
    let m_r = Arc::clone(&m);

    let receiver = thread::spawn(move || {
        let mut count = 0;
        for _ in 0..2 {
            // Wait until channel is full, then take value.
            let permit = full_r.acquire();
            let v = {
                let mut s = slot_r.lock().unwrap();
                s.take().unwrap()
            };
            permit.release();
            empty_r.acquire().release();

            // Acquire shared lock m, but not while waiting on channel.
            {
                let _g = m_r.lock().unwrap();
            }

            if v == 2 {
                count = 1;
            }
        }
        println!("DONE done={}", count);
    });

    sender.join().unwrap();
    receiver.join().unwrap();
}
