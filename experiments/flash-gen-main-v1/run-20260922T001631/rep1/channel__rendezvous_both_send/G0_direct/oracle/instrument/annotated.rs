mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let channel = Arc::new(Channel::new());

    let sender_channel = Arc::clone(&channel);
    let sender = cir_trace::spawn("sender", move || {
        sender_channel.send(1u32);
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver = cir_trace::spawn("receiver", move || receiver_channel.recv());

    sender.join().unwrap();
    let value = receiver.join().unwrap();

    let empty = channel.slot.lock().unwrap().is_none();
    assert!(empty);

    println!("DONE done={}", value);
 cir_trace::finish();}
