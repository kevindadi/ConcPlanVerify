use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // ch: bounded channel of capacity 1 (R2, R4, R7)
    let ch = Arc::new((Mutex::<Option<i32>>::new(None), Condvar::new()));
    // m: the shared lock used occasionally by both roles (R2, R5)
    let m = Arc::new(Mutex::new(()));

    let (ch_s, m_s) = (Arc::clone(&ch), Arc::clone(&m));
    let sender = thread::spawn(move || {
        for v in 1..=2 {
            let (lock, cv) = &*ch_s;
            // Wait for a free slot WITHOUT holding m (R5).
            // R7: slot is Some until the receiver takes v=1, so v=2
            // cannot be sent before the first value is consumed.
            {
                let mut slot = lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                while slot.is_some() {
                    slot = cv
                        .wait(slot)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                *slot = Some(v);
            } // release ch lock before notifying / touching m
            cv.notify_all();
            // Brief use of m, never held while waiting (R5)
            {
                let _g = m_s.lock().unwrap();
            }
        }
    });

    let (ch_r, m_r) = (Arc::clone(&ch), Arc::clone(&m));
    let receiver = thread::spawn(move || {
        let mut got = 0;
        for _ in 0..2 {
            let (lock, cv) = &*ch_r;
            {
                // Wait for a value WITHOUT holding m (R5)
                let mut slot = lock
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                while slot.is_none() {
                    slot = cv
                        .wait(slot)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                if slot.take().is_some() {
                    got += 1;
                }
            } // release ch lock before notifying / touching m
            cv.notify_all();
            {
                let _g = m_r.lock().unwrap();
            }
        }
        got
    });

    sender.join().unwrap();
    let got = receiver.join().unwrap();

    // done is a completion flag: both values received => done == 1 (R3, R8)
    let done = (got == 2) as i32;
    println!("DONE done={}", done);
}
