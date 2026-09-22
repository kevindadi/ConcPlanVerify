use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// A bounded channel that can hold at most one value.
struct Channel {
    slot: Option<i32>,
    // condvar for senders waiting for space
    can_send: Condvar,
    // condvar for receivers waiting for data
    can_recv: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: None,
            can_send: Condvar::new(),
            can_recv: Condvar::new(),
        }
    }

    // Send a value, blocking while the channel is full.
    fn send(&self, value: i32) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            slot = self.can_send.wait(slot).unwrap();
        }
        *slot = Some(value);
        self.can_recv.notify_one();
    }

    // Receive a value, blocking while the channel is empty.
    fn recv(&self) -> i32 {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            slot = self.can_recv.wait(slot).unwrap();
        }
        let value = slot.take().unwrap();
        self.can_send.notify_one();
        value
    }
}

// The channel's internal mutex is the only lock used for the channel.
// The shared lock `m` is a separate mutex that both roles occasionally need.
// To satisfy R5, we never hold `m` while waiting on the channel.

fn sender(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // First value: take the shared lock briefly, then release it before sending.
    {
        let _guard = m.lock().unwrap();
        // do some work under the lock
    }
    ch.send(1);

    // Second value: again take the shared lock briefly, release, then send.
    {
        let _guard = m.lock().unwrap();
        // do some work under the lock
    }
    ch.send(2);
}

fn receiver(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // Receive first value.
    let _v1 = ch.recv();

    // Take the shared lock briefly after receiving.
    {
        let _guard = m.lock().unwrap();
        // do some work under the lock
    }

    // Receive second value.
    let _v2 = ch.recv();

    // Take the shared lock briefly after receiving.
    {
        let _guard = m.lock().unwrap();
        // do some work under the lock
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
