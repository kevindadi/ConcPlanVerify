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

fn main() {
    let ch1: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());
    let ch2: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());

    let shared_lock = Arc::new(Mutex::new(0i32));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let lock_s = Arc::clone(&shared_lock);

    let sender = thread::spawn(move || {
        // Occasionally use the shared lock.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Exchange a value over ch1 (rendezvous).
        ch1_s.send(1);

        // Occasionally use the shared lock again.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Receive the acknowledgement over ch2.
        let _ack = ch2_s.recv();
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let lock_r = Arc::clone(&shared_lock);

    let receiver = thread::spawn(move || {
        // Occasionally use the shared lock.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Receive the value over ch1.
        let v = ch1_r.recv();

        // Occasionally use the shared lock again.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Send acknowledgement over ch2.
        ch2_r.send(v);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
