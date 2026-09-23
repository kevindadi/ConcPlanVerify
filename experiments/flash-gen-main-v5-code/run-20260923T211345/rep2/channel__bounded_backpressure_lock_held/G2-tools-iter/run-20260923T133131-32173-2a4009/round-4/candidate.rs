use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

// A bounded channel with capacity 1, built from two semaphores.
struct Channel {
    // counts empty slots (initially 1)
    empty: Arc<Semaphore>,
    // counts filled slots (initially 0)
    full: Arc<Semaphore>,
    // the single slot
    slot: Mutex<Option<i32>>,
}

impl Channel {
    fn new() -> Self {
        Channel {
            empty: Semaphore::new(1),
            full: Semaphore::new(0),
            slot: Mutex::new(None),
        }
    }

    fn send(&self, value: i32) {
        // Wait for an empty slot before sending.
        let empty_permit = self.empty.acquire();
        {
            let mut slot = self.slot.lock().unwrap();
            *slot = Some(value);
        }
        // Signal that a value is available.
        self.full.acquire().release();
        drop(empty_permit);
    }

    fn recv(&self) -> i32 {
        // Wait for a filled slot before receiving.
        let full_permit = self.full.acquire();
        let value = {
            let mut slot = self.slot.lock().unwrap();
            slot.take().unwrap()
        };
        // Signal that a slot is now empty.
        self.empty.acquire().release();
        drop(full_permit);
        value
    }
}

fn sender(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // First value.
    {
        let _guard = m.lock().unwrap();
        // do some work under the lock, but do not wait on the channel here
    }
    ch.send(1);

    // Second value.
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2);
}

fn receiver(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // Take first value.
    let _v1 = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }

    // Take second value.
    let _v2 = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }
}

fn main() {
    let ch = Arc::new(Channel::new());
    let m = Arc::new(Mutex::new(()));

    let ch_s = Arc::clone(&ch);
    let m_s = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(ch_s, m_s));

    let ch_r = Arc::clone(&ch);
    let m_r = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || receiver(ch_r, m_r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
