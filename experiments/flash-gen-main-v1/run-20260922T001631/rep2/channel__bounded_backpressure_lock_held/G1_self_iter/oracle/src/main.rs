mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.cond.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.cond.notify_all();
    }

    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.cond.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.cond.notify_all();
        value
    }
}

fn main() { cir_trace::init();
    let channel = Arc::new(Channel::new());
    let lock = Arc::new(Mutex::new_named("lock_mutex0", ()));

    let sender_channel = Arc::clone(&channel);
    let sender_lock = Arc::clone(&lock);
    let sender = cir_trace::spawn("sender", move || {
        for i in 1..=2 {
            {
                let _guard = sender_lock.lock().unwrap();
            }
            sender_channel.send(i);
        }
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver_lock = Arc::clone(&lock);
    let receiver = cir_trace::spawn("receiver", move || {
        let mut done = 0;
        for _ in 1..=2 {
            {
                let _guard = receiver_lock.lock().unwrap();
            }
            let _value = receiver_channel.recv();
            done += 1;
        }
        println!("DONE done={}", done);
    });

    sender.join().unwrap();
    receiver.join().unwrap();
 cir_trace::finish();}
