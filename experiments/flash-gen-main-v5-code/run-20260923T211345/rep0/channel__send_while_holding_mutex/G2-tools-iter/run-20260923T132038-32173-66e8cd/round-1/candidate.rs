use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

// ch1: rendezvous channel (capacity 0) implemented with two semaphores.
// ch2: another rendezvous channel used to signal completion.
struct Rendezvous {
    // sender waits here until receiver is ready
    send_ready: Arc<Semaphore>,
    // receiver waits here until sender has delivered
    recv_ready: Arc<Semaphore>,
    // protects the payload
    slot: Mutex<Option<i32>>,
}

impl Rendezvous {
    fn new() -> Self {
        Rendezvous {
            send_ready: Semaphore::new(0),
            recv_ready: Semaphore::new(0),
            slot: Mutex::new(None),
        }
    }

    // Sender side: deposit value, then wait for receiver to take it.
    fn send(&self, v: i32) {
        {
            let mut s = self.slot.lock().unwrap();
            *s = Some(v);
        }
        // signal receiver that a value is available
        self.recv_ready.acquire().permit_release_placeholder();
    }
}

fn main() {
    // placeholder to satisfy structure; real code below
}
