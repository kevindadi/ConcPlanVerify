use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel<T> {
    slot: Mutex<Option<T>>,
    has_value: Condvar,
    has_space: Condvar,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            has_value: Condvar::new(),
            has_space: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.has_space.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.has_value.notify_one();
    }

    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.has_value.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.has_space.notify_one();
        value
    }
}

fn main() {
    let ch: Arc<Channel<i32>> = Arc::new(Channel::new());

    let ch_s1 = Arc::clone(&ch);
    let s1 = thread::spawn(move || {
        ch_s1.send(1);
    });

    let ch_r = Arc::clone(&ch);
    let r = thread::spawn(move || {
        let _value = ch_r.recv();
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
