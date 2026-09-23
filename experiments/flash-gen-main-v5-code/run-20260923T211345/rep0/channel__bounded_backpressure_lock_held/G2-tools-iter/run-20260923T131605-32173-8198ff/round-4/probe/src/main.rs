use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

struct Channel<T> {
    slot: Option<T>,
    empty: Arc<Semaphore>,
    full: Arc<Semaphore>,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            slot: None,
            empty: Semaphore::new(1),
            full: Semaphore::new(0),
        }
    }

    fn send(&mut self, value: T) {
        // Wait for an empty slot before touching the slot.
        let permit = self.empty.acquire();
        self.slot = Some(value);
        // The slot is now filled: release the empty permit only after the
        // receiver has taken the value, so keep it and signal "full" instead.
        permit.release();
        self.full.acquire().release();
    }

    fn recv(&mut self) -> T {
        // Wait for a filled slot before touching the slot.
        let permit = self.full.acquire();
        let value = self.slot.take().unwrap();
        // The slot is now empty: release the filled permit and signal "empty".
        permit.release();
        self.empty.acquire().release();
        value
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let ch = Arc::new(Mutex::new(Channel::<i32>::new()));

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);

    let sender = thread::spawn(move || {
        // Acquire the shared lock, then release it before using the channel.
        {
            let _guard = m_sender.lock().unwrap();
        }
        {
            let mut c = ch_sender.lock().unwrap();
            c.send(1);
        }
        {
            let _guard = m_sender.lock().unwrap();
        }
        {
            let mut c = ch_sender.lock().unwrap();
            c.send(2);
        }
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);

    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let v1 = {
            let mut c = ch_receiver.lock().unwrap();
            c.recv()
        };
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let v2 = {
            let mut c = ch_receiver.lock().unwrap();
            c.recv()
        };
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
