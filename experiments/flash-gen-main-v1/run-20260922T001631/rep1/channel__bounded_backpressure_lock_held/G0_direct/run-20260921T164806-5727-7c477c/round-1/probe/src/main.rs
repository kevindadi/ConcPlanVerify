use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    slot: Mutex<Option<i32>>,
    empty: Condvar,
    full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            empty: Condvar::new(),
            full: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_some() {
            guard = self.empty.wait(guard).unwrap();
        }
        *guard = Some(value);
        self.full.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_none() {
            guard = self.full.wait(guard).unwrap();
        }
        let value = guard.take().unwrap();
        self.empty.notify_one();
        value
    }
}

fn main() {
    let channel = Arc::new(Channel::new());
    let shared_lock = Arc::new(Mutex::new(0i32));

    let sender_channel = Arc::clone(&channel);
    let sender_lock = Arc::clone(&shared_lock);
    let sender = thread::spawn(move || {
        for value in [1, 2] {
            {
                let mut guard = sender_lock.lock().unwrap();
                *guard += value;
            }
            sender_channel.send(value);
        }
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver_lock = Arc::clone(&shared_lock);
    let receiver = thread::spawn(move || {
        let mut done = 0i32;
        for _ in 0..2 {
            let value = receiver_channel.recv();
            {
                let mut guard = receiver_lock.lock().unwrap();
                *guard += value;
            }
            done += 1;
        }
        done
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
}
