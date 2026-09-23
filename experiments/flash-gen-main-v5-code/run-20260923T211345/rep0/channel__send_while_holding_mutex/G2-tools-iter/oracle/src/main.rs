mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

// ch1: rendezvous channel (capacity 0) implemented with two semaphores.
// ch2: another rendezvous channel used to signal completion.
//
// Each semaphore is pre-loaded with one permit ("token"). The sender starts
// holding the recv_ready token and the receiver starts holding the send_ready
// token. A send hands its token to the receiver; a recv hands its token back.
// This makes the two roles meet without either blocking while holding the
// shared lock.
struct Rendezvous {
    // token initially held by the receiver; sender waits on it after delivering
    send_ready: Arc<Semaphore>,
    // token initially held by the sender; receiver waits on it for a value
    recv_ready: Arc<Semaphore>,
    // protects the payload
    slot: Mutex<Option<i32>>,
}

impl Rendezvous {
    fn new() -> Self {
        Rendezvous {
            send_ready: Semaphore::new(1),
            recv_ready: Semaphore::new(1),
            slot: Mutex::new(None),
        }
    }

    // Sender side: deposit value, then wait for receiver to take it.
    fn send(&self, v: i32) {
        // Take the recv_ready token (available initially).
        let rp = self.recv_ready.acquire();
        {
            let mut s = self.slot.lock().unwrap();
            *s = Some(v);
        }
        // Hand the token to the receiver: a value is available.
        rp.release();
        // Wait until the receiver has taken the value.
        let sp = self.send_ready.acquire();
        // Hand the token back so the channel can be reused.
        sp.release();
    }

    // Receiver side: wait for a value, take it, then signal sender.
    fn recv(&self) -> i32 {
        // Take the send_ready token (available initially).
        let sp = self.send_ready.acquire();
        // Wait until a value is available.
        let rp = self.recv_ready.acquire();
        let v = {
            let mut s = self.slot.lock().unwrap();
            s.take().unwrap()
        };
        // Hand the recv_ready token back: the value has been taken.
        rp.release();
        // Hand the send_ready token back: the sender may proceed.
        sp.release();
        v
    }
}

fn main() { cir_trace::init();
    let ch1 = Arc::new(Rendezvous::new());
    let ch2 = Arc::new(Rendezvous::new());
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0));

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
        // signal completion over ch2
        cir_trace::record("channel_send", "ch2_s"); ch2_s.send(1);
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
        // receive completion over ch2
        cir_trace::record("channel_recv", "ch2_r"); let _ = ch2_r.recv();
        v
    });

    s.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
