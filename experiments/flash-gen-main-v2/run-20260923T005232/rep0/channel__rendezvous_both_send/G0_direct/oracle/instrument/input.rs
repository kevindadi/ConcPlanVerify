use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel<T> {
    slot: Mutex<Option<T>>,
    cond: Condvar,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            cond: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_some() {
            guard = self.cond.wait(guard).unwrap();
        }
        *guard = Some(value);
        self.cond.notify_all();
    }

    fn recv(&self) -> T {
        let mut guard = self.slot.lock().unwrap();
        loop {
            if let Some(value) = guard.take() {
                self.cond.notify_all();
                return value;
            }
            guard = self.cond.wait(guard).unwrap();
        }
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
        let value = ch_r.recv();
        value
    });

    s1.join().unwrap();
    let done = r.join().unwrap();

    println!("DONE done={}", done);
}
