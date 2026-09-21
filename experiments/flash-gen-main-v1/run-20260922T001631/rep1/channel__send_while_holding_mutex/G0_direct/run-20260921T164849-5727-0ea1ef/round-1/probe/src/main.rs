use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A rendezvous channel: sender and receiver must meet.
struct Rendezvous<T> {
    slot: Mutex<Option<T>>,
    // Signaled when the slot becomes full (for receiver) or empty (for sender).
    full: Condvar,
    empty: Condvar,
}

impl<T> Rendezvous<T> {
    fn new() -> Self {
        Rendezvous {
            slot: Mutex::new(None),
            full: Condvar::new(),
            empty: Condvar::new(),
        }
    }

    // Sender: wait until slot is empty, put value, signal full.
    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.empty.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.full.notify_one();
    }

    // Receiver: wait until slot is full, take value, signal empty.
    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.full.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.empty.notify_one();
        value
    }
}

fn main() {
    let channel: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());
    // Shared lock that both roles occasionally use.
    let shared_lock: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    let sender_channel = Arc::clone(&channel);
    let sender_lock = Arc::clone(&shared_lock);

    let sender = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut guard = sender_lock.lock().unwrap();
            *guard += 1;
        }
        // Send over the rendezvous channel without holding the shared lock.
        sender_channel.send(1);
    });

    let receiver_channel = Arc::clone(&channel);
    let receiver_lock = Arc::clone(&shared_lock);

    let receiver = thread::spawn(move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut guard = receiver_lock.lock().unwrap();
            *guard += 1;
        }
        // Receive over the rendezvous channel without holding the shared lock.
        let value = receiver_channel.recv();
        assert_eq!(value, 1);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
