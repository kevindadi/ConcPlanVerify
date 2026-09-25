use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// A rendezvous (unbuffered) channel: a send and a take must meet.
struct Channel {
    // Some(value) means a value is waiting to be taken.
    slot: Mutex<Option<i32>>,
    // Signaled when a value is placed or taken.
    cv: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            cv: Condvar::new(),
        }
    }

    // Blocking send: waits until the receiver takes the value.
    fn send(&self, value: i32) {
        let mut slot = self.slot.lock().unwrap();
        // Wait until the slot is empty (previous value taken).
        while slot.is_some() {
            slot = self.cv.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.cv.notify_all();
        // Wait until the value is taken.
        while slot.is_some() {
            slot = self.cv.wait(slot).unwrap();
        }
    }

    // Blocking take: waits until a value is available, then takes it.
    fn take(&self) -> i32 {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.cv.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.cv.notify_all();
        value
    }

    fn is_empty(&self) -> bool {
        self.slot.lock().unwrap().is_none()
    }
}

fn main() {
    let ch = Arc::new(Channel::new());

    let ch_s = Arc::clone(&ch);
    let s1 = thread::spawn(move || {
        ch_s.send(1);
    });

    let ch_r = Arc::clone(&ch);
    let r = thread::spawn(move || {
        let _v = ch_r.take();
    });

    s1.join().unwrap();
    r.join().unwrap();

    assert!(ch.is_empty());
    println!("DONE done=1");
}
