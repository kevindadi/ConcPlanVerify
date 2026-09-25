mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

// ch1: rendezvous channel (capacity 0) implemented with two semaphores.
// ch2: shared lock (mutex) used occasionally by both roles.

struct Rendezvous<T> {
    send: Arc<Semaphore>,
    recv: Arc<Semaphore>,
    slot: Mutex<Option<T>>,
}

impl<T> Rendezvous<T> {
    fn new() -> Self {
        Rendezvous {
            send: Semaphore::new(0),
            recv: Semaphore::new(0),
            slot: Mutex::new(None),
        }
    }

    fn send(&self, value: T) {
        // Wait until receiver is ready to take the value.
        let _ready = self.recv.acquire();
        *self.slot.lock().unwrap() = Some(value);
        // Signal that a value is available.
        self.send.acquire().release();
    }

    fn recv(&self) -> T {
        // Signal sender that we are ready.
        self.recv.acquire().release();
        // Wait for the value.
        let _got = self.send.acquire();
        self.slot.lock().unwrap().take().unwrap()
    }
}

fn main() { cir_trace::init();
    let ch1: Arc<Rendezvous<i32>> = Arc::new(Rendezvous::new());
    let ch2 = Arc::new(Mutex::new_named("ch2_mutex0", 0i32));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock.
        {
            let mut g = ch2_s.lock().unwrap();
            *g += 1;
        }
        // Send over the rendezvous channel without holding ch2.
        cir_trace::record("channel_send", "ch1_s"); ch1_s.send(1);
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock.
        {
            let mut g = ch2_r.lock().unwrap();
            *g += 1;
        }
        // Receive over the rendezvous channel without holding ch2.
        cir_trace::record("channel_recv", "ch1_r"); let v = ch1_r.recv();
        assert_eq!(v, 1);
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
