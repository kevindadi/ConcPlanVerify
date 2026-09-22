use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    buffer: Mutex<Option<i32>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            buffer: Mutex::new(None),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, value: i32) {
        let mut slot = self.buffer.lock().unwrap();
        while slot.is_some() {
            slot = self.not_full.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut slot = self.buffer.lock().unwrap();
        while slot.is_none() {
            slot = self.not_empty.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let ch = Arc::new(Channel::new());

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);
    let sender = thread::spawn(move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.send(1);
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.send(2);
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);
    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let x = ch_receiver.recv();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let y = ch_receiver.recv();
        let _ = (x, y);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
