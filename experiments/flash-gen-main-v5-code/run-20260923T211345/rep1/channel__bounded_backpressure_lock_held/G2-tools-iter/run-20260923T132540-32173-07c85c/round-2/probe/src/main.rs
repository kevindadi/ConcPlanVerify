use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

// A bounded channel with capacity 1, built from two semaphores.
struct Channel {
    // Counts available slots (initially 1).
    empty: Arc<Semaphore>,
    // Counts available items (initially 0).
    full: Arc<Semaphore>,
    // The single buffered value.
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

    // Send a value, waiting if the channel is full.
    // The shared lock `m` is NOT held while waiting.
    fn send(&self, value: i32) {
        // Wait for an empty slot before touching the buffer.
        let slot_permit = self.empty.acquire();
        {
            let mut guard = self.slot.lock().unwrap();
            *guard = Some(value);
        }
        // Signal a full item, then release the empty slot.
        self.full.acquire().release();
        slot_permit.release();
    }

    // Receive a value, waiting if the channel is empty.
    // The shared lock `m` is NOT held while waiting.
    fn recv(&self) -> i32 {
        // Wait for a full slot before touching the buffer.
        let item_permit = self.full.acquire();
        let value = {
            let mut guard = self.slot.lock().unwrap();
            guard.take().unwrap()
        };
        // Signal an empty slot, then release the item permit.
        self.empty.acquire().release();
        item_permit.release();
        value
    }
}

fn sender(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // First value.
    {
        let _guard = m.lock().unwrap();
        // hold lock briefly, then release before channel wait
    }
    ch.send(1);

    // Second value: only after receiver took the first (channel capacity 1).
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2);
}

fn receiver(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    let a = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }
    let b = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }
    // Ensure both values were received.
    assert_eq!(a, 1);
    assert_eq!(b, 2);
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
