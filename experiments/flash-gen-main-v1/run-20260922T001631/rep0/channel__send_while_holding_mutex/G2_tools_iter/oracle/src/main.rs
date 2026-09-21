mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// A rendezvous channel: sender and receiver must meet.
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

    // Sender: place value, wait until receiver takes it.
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

    // Receiver: wait until value present, take it.
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
    let chan = Arc::new(Rendezvous::new());

    let shared_s = Arc::clone(&shared);
    let chan_s = Arc::clone(&chan);
    let sender = cir_trace::spawn("sender", move || {
        // Occasionally use the shared lock, but never while waiting on the channel.
        {
            let mut g = shared_s.lock().unwrap();
            *g += 1;
        }
        chan_s.send(1u64);
        {
            let mut g = shared_s.lock().unwrap();
            *g += 1;
        }
    });

    let shared_r = Arc::clone(&shared);
    let chan_r = Arc::clone(&chan);
    let receiver = cir_trace::spawn("receiver", move || {
        {
            let mut g = shared_r.lock().unwrap();
            *g += 1;
        }
        let v = chan_r.recv();
        {
            let mut g = shared_r.lock().unwrap();
            *g += 1;
        }
        v
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
 cir_trace::finish();}
