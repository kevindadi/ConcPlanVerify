mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", 0u64));
    let rendezvous = Arc::new(Rendezvous::new());

    let shared_sender = Arc::clone(&shared);
    let rendezvous_sender = Arc::clone(&rendezvous);
    let sender = cir_trace::spawn("sender", move || {
        {
            let mut guard = shared_sender.lock().unwrap();
            *guard += 1;
        }
        rendezvous_sender.send(1u64);
    });

    let shared_receiver = Arc::clone(&shared);
    let rendezvous_receiver = Arc::clone(&rendezvous);
    let receiver = cir_trace::spawn("receiver", move || {
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
 cir_trace::finish();}
