use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A bounded channel with capacity 1, plus a shared lock `m`.
// The channel has its own internal mutex/condvar so that waiting on the
// channel never holds the shared lock `m`.
struct Channel<T> {
    slot: Mutex<Option<T>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.not_full.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.not_empty.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn main() {
    let ch = Arc::new(Channel::new());
    let m = Arc::new(Mutex::new(()));

    let ch_s = Arc::clone(&ch);
    let m_s = Arc::clone(&m);
    let sender = thread::spawn(move || {
        // First value: take shared lock briefly, then send without holding it.
        {
            let _guard = m_s.lock().unwrap();
        }
        ch_s.send(1);

        // Second value: again take shared lock briefly, then send.
        {
            let _guard = m_s.lock().unwrap();
        }
        ch_s.send(2);
    });

    let ch_r = Arc::clone(&ch);
    let m_r = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        let a = ch_r.recv();
        {
            let _guard = m_r.lock().unwrap();
        }
        let b = ch_r.recv();
        {
            let _guard = m_r.lock().unwrap();
        }
        a + b
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();
    println!("DONE done={}", done);
}
