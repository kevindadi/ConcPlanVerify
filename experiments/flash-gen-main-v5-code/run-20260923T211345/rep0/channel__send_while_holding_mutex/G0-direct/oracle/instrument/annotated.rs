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
        let mut guard = self.slot.lock().unwrap();
        while guard.is_some() {
            guard = self.cv.wait(guard).unwrap();
        }
        *guard = Some(value);
        self.cv.notify_all();
        while guard.is_some() {
            guard = self.cv.wait(guard).unwrap();
        }
    }

    // Receiver: wait for value, take it.
    fn recv(&self) -> T {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_none() {
            guard = self.cv.wait(guard).unwrap();
        }
        let value = guard.take().unwrap();
        self.cv.notify_all();
        value
    }
}

fn main() { cir_trace::init();
    let ch1: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());
    let ch2: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());

    // Shared lock used occasionally by both roles.
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0", 0u32));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let lock_s = Arc::clone(&shared_lock);

    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Send over ch1 (rendezvous, not holding shared lock).
        cir_trace::record("channel_send", "ch1_s"); ch1_s.send(1);

        // Occasionally use the shared lock again.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Receive over ch2 (rendezvous, not holding shared lock).
        cir_trace::record("channel_recv", "ch2_s"); let v = ch2_s.recv();
        assert_eq!(v, 1);
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

        // Receive over ch1 (rendezvous, not holding shared lock).
        cir_trace::record("channel_recv", "ch1_r"); let v = ch1_r.recv();
        assert_eq!(v, 1);

        // Occasionally use the shared lock again.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Send over ch2 (rendezvous, not holding shared lock).
        cir_trace::record("channel_send", "ch2_r"); ch2_r.send(1);
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
