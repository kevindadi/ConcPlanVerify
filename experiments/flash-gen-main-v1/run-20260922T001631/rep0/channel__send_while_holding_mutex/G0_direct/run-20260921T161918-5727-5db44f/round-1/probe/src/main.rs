use std::sync::{Arc, Condvar, Mutex};
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

    // Sender: wait until receiver is ready, then deposit value.
    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        // Wait until the slot is empty (receiver has taken previous value or is ready).
        while slot.is_some() {
            slot = self.receiver_ready.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.sender_ready.notify_one();
        // Wait until receiver takes it.
        while slot.is_some() {
            slot = self.receiver_ready.wait(slot).unwrap();
        }
    }

    // Receiver: wait until sender deposits, then take value.
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

fn main() {
    let shared_lock = Arc::new(Mutex::new(0u64));
    let channel = Arc::new(Rendezvous::new());

    let shared_lock_s = Arc::clone(&shared_lock);
    let channel_s = Arc::clone(&channel);

    let sender = thread::spawn(move || {
        // Occasionally use the shared lock.
        {
            let mut g = shared_lock_s.lock().unwrap();
            *g += 1;
        }
        // Send value over rendezvous channel (not holding shared lock).
        channel_s.send(1u64);
        // Occasionally use the shared lock again.
        {
            let mut g = shared_lock_s.lock().unwrap();
            *g += 1;
        }
    });

    let shared_lock_r = Arc::clone(&shared_lock);
    let channel_r = Arc::clone(&channel);

    let receiver = thread::spawn(move || {
        // Occasionally use the shared lock.
        {
            let mut g = shared_lock_r.lock().unwrap();
            *g += 1;
        }
        // Receive value over rendezvous channel (not holding shared lock).
        let value = channel_r.recv();
        // Occasionally use the shared lock again.
        {
            let mut g = shared_lock_r.lock().unwrap();
            *g += 1;
        }
        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
}
