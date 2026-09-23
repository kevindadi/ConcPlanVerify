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
        self.recv_ready.acquire().release();
        // wait until receiver has taken the value
        self.send_ready.acquire().release();
    }

    // Receiver side: wait for a value, take it, then signal sender.
    fn recv(&self) -> i32 {
        // wait until a value is available
        self.recv_ready.acquire().release();
        let v = {
            let mut s = self.slot.lock().unwrap();
            s.take().unwrap()
        };
        // signal sender that the value has been taken
        self.send_ready.acquire().release();
        v
    }
}

fn main() {
    let ch1 = Arc::new(Rendezvous::new());
    let ch2 = Arc::new(Rendezvous::new());
    let lock = Arc::new(Mutex::new(0));

    let ch1_s = Arc::clone(&ch1);
    let ch2_s = Arc::clone(&ch2);
    let lock_s = Arc::clone(&lock);

    let s = thread::spawn(move || {
        // occasionally use the shared lock
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }
        // send over ch1 without holding the lock
        ch1_s.send(1);
        // signal completion over ch2
        ch2_s.send(1);
    });

    let ch1_r = Arc::clone(&ch1);
    let ch2_r = Arc::clone(&ch2);
    let lock_r = Arc::clone(&lock);

    let r = thread::spawn(move || {
        // occasionally use the shared lock
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }
        // receive over ch1 without holding the lock
        let v = ch1_r.recv();
        // receive completion over ch2
        let _ = ch2_r.recv();
        v
    });

    s.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
}
