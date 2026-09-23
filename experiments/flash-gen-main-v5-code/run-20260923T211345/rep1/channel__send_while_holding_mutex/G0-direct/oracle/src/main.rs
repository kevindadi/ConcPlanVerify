mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

// A rendezvous channel: sender and receiver must meet.
struct Rendezvous<T> {
    slot: Mutex<Option<T>>,
    // sender waits until receiver takes the value
    sender_ready: Semaphore,
    // receiver waits until sender has put a value
    receiver_ready: Semaphore,
}

impl<T> Rendezvous<T> {
    fn new() -> Arc<Self> {
        Arc::new(Rendezvous {
            slot: Mutex::new(None),
            sender_ready: Semaphore::new(0),
            receiver_ready: Semaphore::new(0),
        })
    }

    fn send(&self, value: T) {
        {
            let mut slot = self.slot.lock().unwrap();
            *slot = Some(value);
        }
        // signal receiver that a value is available
        self.receiver_ready.acquire().permit_release_placeholder();
    }

    fn recv(&self) -> T {
        // wait for a value
        let permit = self.receiver_ready.acquire();
        let value = {
            let mut slot = self.slot.lock().unwrap();
            slot.take().unwrap()
        };
        // signal sender that the value was taken
        drop(permit);
        self.sender_ready.acquire().permit_release_placeholder();
        value
    }
}

// Helper trait to release a permit immediately (placeholder to satisfy API usage).
trait PermitRelease {
    fn permit_release_placeholder(self);
}

fn main() { cir_trace::init();
    let ch1 = Rendezvous::<i32>::new();
    let ch2 = Rendezvous::<i32>::new();
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0i32));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let lock_s = Arc::clone(&lock);

    let s = cir_trace::spawn("s", move || {
        // occasionally use the shared lock
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
        // send over ch1 without holding the lock
        cir_trace::record("channel_send", "ch1_s"); ch1_s.send(1);
        // receive over ch2 without holding the lock
        cir_trace::record("channel_recv", "ch2_s"); let v = ch2_s.recv();
        // occasionally use the shared lock again
        {
            let mut g = lock_s.lock().unwrap();
            *g += v;
        }
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let lock_r = Arc::clone(&lock);

    let r = cir_trace::spawn("r", move || {
        // occasionally use the shared lock
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }
        // receive over ch1 without holding the lock
        cir_trace::record("channel_recv", "ch1_r"); let v = ch1_r.recv();
        // send over ch2 without holding the lock
        cir_trace::record("channel_send", "ch2_r"); ch2_r.send(v);
        // occasionally use the shared lock again
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
