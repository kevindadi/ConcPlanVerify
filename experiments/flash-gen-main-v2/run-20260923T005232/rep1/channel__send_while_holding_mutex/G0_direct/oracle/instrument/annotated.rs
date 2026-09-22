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
    let ch1: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());
    let ch2: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());

    // Shared lock that both roles occasionally use.
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0", 0u32));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let lock_s = Arc::clone(&shared_lock);

    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock, but never while waiting on channel.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
        // Send over ch1 (rendezvous with receiver).
        ch1_s.send(1);
        // Receive ack over ch2.
        let _ack = ch2_s.recv();
        // Use shared lock again.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let lock_r = Arc::clone(&shared_lock);

    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }
        // Receive over ch1.
        let v = ch1_r.recv();
        // Use shared lock again.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }
        // Send ack over ch2.
        ch2_r.send(v);
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
