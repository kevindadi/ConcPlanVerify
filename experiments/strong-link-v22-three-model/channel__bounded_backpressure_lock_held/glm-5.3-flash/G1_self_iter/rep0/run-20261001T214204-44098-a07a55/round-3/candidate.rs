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
            // Wait for free slot WITHOUT touching m (R5)
            // R7: with capacity 1, sending v=2 can only proceed after the
            // receiver has taken v=1 and freed the slot via notify_all.
            let mut slot = lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while slot.is_some() {
                slot = cv
                    .wait(slot)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            // Brief use of m after releasing any wait; not held while waiting (R5)
            {
                let _g = m_s.lock().unwrap();
            }
            *slot = Some(v);
            cv.notify_all();
        }
    });

    let (ch_r, m_r) = (Arc::clone(&ch), Arc::clone(&m));
    let receiver = thread::spawn(move || {
        let mut got = 0;
        for _ in 0..2 {
            let (lock, cv) = &*ch_r;
            // Wait for value WITHOUT touching m (R5)
            let mut slot = lock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while slot.is_none() {
                slot = cv
                    .wait(slot)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            {
                let _g = m_r.lock().unwrap();
            }
            if slot.take().is_some() {
                got += 1;
            }
            cv.notify_all();
        }
        got
    });

    sender.join().unwrap();
    let got = receiver.join().unwrap();

    // done is a completion flag, not the raw receive count:
    // both values received (got == 2) means done == 1 (R3, R8).
    let done = (got == 2) as i32;
    println!("DONE done={}", done); // R8: exactly "DONE done=1"
}
