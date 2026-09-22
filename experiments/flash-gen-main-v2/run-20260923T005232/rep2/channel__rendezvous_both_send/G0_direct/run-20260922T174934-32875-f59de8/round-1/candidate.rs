use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    slot: Option<i32>,
    sender_waiting: bool,
    receiver_waiting: bool,
}

struct Shared {
    state: Mutex<Channel>,
    cv: Condvar,
}

impl Shared {
    fn new() -> Self {
        Shared {
            state: Mutex::new(Channel {
                slot: None,
                sender_waiting: false,
                receiver_waiting: false,
            }),
            cv: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut guard = self.state.lock().unwrap();
        while guard.slot.is_some() {
            guard.sender_waiting = true;
            guard = self.cv.wait(guard).unwrap();
            guard.sender_waiting = false;
        }
        guard.slot = Some(value);
        self.cv.notify_all();
        while guard.slot.is_some() {
            guard.sender_waiting = true;
            guard = self.cv.wait(guard).unwrap();
            guard.sender_waiting = false;
        }
    }

    fn recv(&self) -> i32 {
        let mut guard = self.state.lock().unwrap();
        while guard.slot.is_none() {
            guard.receiver_waiting = true;
            guard = self.cv.wait(guard).unwrap();
            guard.receiver_waiting = false;
        }
        let value = guard.slot.take().unwrap();
        self.cv.notify_all();
        value
    }
}

fn main() {
    let shared = Arc::new(Shared::new());

    let s_shared = Arc::clone(&shared);
    let s1 = thread::spawn(move || {
        s_shared.send(42);
    });

    let r_shared = Arc::clone(&shared);
    let r = thread::spawn(move || {
        let _value = r_shared.recv();
    });

    s1.join().unwrap();
    r.join().unwrap();

    let guard = shared.state.lock().unwrap();
    assert!(guard.slot.is_none());

    println!("DONE done=1");
}
