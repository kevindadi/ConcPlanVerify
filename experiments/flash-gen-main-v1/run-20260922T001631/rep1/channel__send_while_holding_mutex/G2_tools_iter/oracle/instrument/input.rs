use std::sync::{Arc, Mutex, Condvar};
use std::thread;

struct Rendezvous<T> {
    slot: Mutex<Option<T>>,
    cv: Condvar,
}

impl<T> Rendezvous<T> {
    fn new() -> Self {
        Rendezvous {
            slot: Mutex::new(None),
            cv: Condvar::new(),
        }
    }

    fn send(&self, value: T) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.cv.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.cv.notify_all();
        while slot.is_some() {
            slot = self.cv.wait(slot).unwrap();
        }
    }

    fn recv(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.cv.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.cv.notify_all();
        value
    }
}

fn main() {
    let shared = Arc::new(Mutex::new(0u64));
    let rendezvous = Arc::new(Rendezvous::new());

    let shared_sender = Arc::clone(&shared);
    let rendezvous_sender = Arc::clone(&rendezvous);

    let sender = thread::spawn(move || {
        {
            let mut guard = shared_sender.lock().unwrap();
            *guard += 1;
        }
        rendezvous_sender.send(1u64);
    });

    let shared_receiver = Arc::clone(&shared);
    let rendezvous_receiver = Arc::clone(&rendezvous);

    let receiver = thread::spawn(move || {
        {
            let mut guard = shared_receiver.lock().unwrap();
            *guard += 1;
        }
        let value = rendezvous_receiver.recv();
        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
}
