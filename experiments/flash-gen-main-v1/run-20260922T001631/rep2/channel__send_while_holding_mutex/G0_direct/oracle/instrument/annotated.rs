mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// A rendezvous channel: sender and receiver must meet.
struct Rendezvous<T> {
    slot: Mutex<Option<T>>,
    sender_ready: Condvar,
    receiver_ready: Condvar,
}

impl<T> Rendezvous<T> {
    fn new() -> Self {
        Rendezvous {
            slot: Mutex::new(None),
            sender_ready: Condvar::new(),
            receiver_ready: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.receiver_ready.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.sender_ready.notify_one();
        while slot.is_some() {
            slot = self.receiver_ready.wait(slot).unwrap();
        }
    }

    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.sender_ready.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.receiver_ready.notify_one();
        value
    }
}

fn main() { cir_trace::init();
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0", 0u64));
    let channel = Arc::new(Rendezvous::new());

    let sender_lock = Arc::clone(&shared_lock);
    let sender_channel = Arc::clone(&channel);

    let sender = cir_trace::spawn("sender", move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut guard = sender_lock.lock().unwrap();
            *guard += 1;
        }

        sender_channel.send(1u64);

        {
            let mut guard = sender_lock.lock().unwrap();
            *guard += 1;
        }
    });

    let receiver_lock = Arc::clone(&shared_lock);
    let receiver_channel = Arc::clone(&channel);

    let receiver = cir_trace::spawn("receiver", move || {
        {
            let mut guard = receiver_lock.lock().unwrap();
            *guard += 1;
        }

        let value = receiver_channel.recv();

        {
            let mut guard = receiver_lock.lock().unwrap();
            *guard += 1;
        }

        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
 cir_trace::finish();}
