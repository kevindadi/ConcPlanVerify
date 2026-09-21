use std::sync::{Arc, Condvar, Mutex};
use std::thread;

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
    let channel = Arc::new(Channel::new());
    let lock = Arc::new(Mutex::new(()));

    let sender_channel = Arc::clone(&channel);
    let sender_lock = Arc::clone(&lock);
    let sender = thread::spawn(move || {
        for i in 0..2 {
            {
                let _guard = sender_lock.lock().unwrap();
            }
            sender_channel.send(i);
        }
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver_lock = Arc::clone(&lock);
    let receiver = thread::spawn(move || {
        let mut done = 0;
        for _ in 0..2 {
            let _value = receiver_channel.recv();
            {
                let _guard = receiver_lock.lock().unwrap();
            }
            done += 1;
        }
        done
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();
    println!("DONE done={}", done);
}
