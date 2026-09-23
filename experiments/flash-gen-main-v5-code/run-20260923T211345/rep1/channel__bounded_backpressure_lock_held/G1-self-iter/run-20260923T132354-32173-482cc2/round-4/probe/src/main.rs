use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn main() {
    // Channel `ch` with capacity 1: a slot guarded by a mutex + condvar.
    let ch = Arc::new((Mutex::new(None::<i32>), Condvar::new()));

    // Shared lock `m`.
    let m = Arc::new(Mutex::new(()));

    let ch_s = ch.clone();
    let m_s = m.clone();

    let sender = thread::spawn(move || {
        for value in [1, 2] {
            // Do some work with the shared lock, but do not wait on the channel here.
            {
                let _guard = m_s.lock().unwrap();
            }

            // Wait for a free slot (channel empty) before sending, not holding `m`.
            let (lock, cvar) = &*ch_s;
            let mut slot = lock.lock().unwrap();
            while slot.is_some() {
                slot = cvar.wait(slot).unwrap();
            }
            *slot = Some(value);
            cvar.notify_all();
        }
    });

    let ch_r = ch.clone();
    let m_r = m.clone();

    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // Wait until the channel is full (an item is available).
            let (lock, cvar) = &*ch_r;
            let mut slot = lock.lock().unwrap();
            while slot.is_none() {
                slot = cvar.wait(slot).unwrap();
            }
            let _value = slot.take().unwrap();
            cvar.notify_all();
            drop(slot);

            // Do some work with the shared lock, but do not wait on the channel here.
            {
                let _guard = m_r.lock().unwrap();
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
