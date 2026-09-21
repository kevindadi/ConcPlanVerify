mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Option<i32>,
    lock: Mutex<()>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: None,
            lock: Mutex::new(()),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut guard = self.lock.lock().unwrap();
        while self.slot.is_some() {
            guard = self.not_full.wait(guard).unwrap();
        }
        self.slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut guard = self.lock.lock().unwrap();
        while self.slot.is_none() {
            guard = self.not_empty.wait(guard).unwrap();
        }
        let value = self.slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn main() { cir_trace::init();
    let channel = Arc::new(Channel::new());
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0", ()));

    let sender_channel = Arc::clone(&channel);
    let sender_lock = Arc::clone(&shared_lock);
    let sender = cir_trace::spawn("sender", move || {
        {
            let _guard = sender_lock.lock().unwrap();
        }
        sender_channel.send(1);
        {
            let _guard = sender_lock.lock().unwrap();
        }
        sender_channel.send(2);
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver_lock = Arc::clone(&shared_lock);
    let receiver = cir_trace::spawn("receiver", move || {
        {
            let _guard = receiver_lock.lock().unwrap();
        }
        let _first = receiver_channel.recv();
        {
            let _guard = receiver_lock.lock().unwrap();
        }
        let _second = receiver_channel.recv();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
