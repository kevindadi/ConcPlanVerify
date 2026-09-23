mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// A bounded channel with capacity 1, built from a mutex + condition variables.
struct Channel {
    // The single buffered value (None = empty).
    state: Mutex<Option<i32>>,
    // Signaled when the channel becomes non-empty.
    not_empty: Condvar,
    // Signaled when the channel becomes non-full (a slot frees up).
    not_full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            state: Mutex::new(None),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    // Send a value, waiting if the channel is full.
    // The shared lock `m` is NOT held while waiting here.
    fn send(&self, value: i32) {
        let mut guard = self.state.lock().unwrap();
        // Wait until there is a free slot (channel not full).
        while guard.is_some() {
            guard = self.not_full.wait(guard).unwrap();
        }
        *guard = Some(value);
        // Wake a receiver waiting for an item.
        self.not_empty.notify_one();
    }

    // Receive a value, waiting if the channel is empty.
    // The shared lock `m` is NOT held while waiting here.
    fn recv(&self) -> i32 {
        let mut guard = self.state.lock().unwrap();
        // Wait until there is an item (channel not empty).
        while guard.is_none() {
            guard = self.not_empty.wait(guard).unwrap();
        }
        let value = guard.take().unwrap();
        // Wake a sender waiting for a free slot.
        self.not_full.notify_one();
        value
    }
}

fn sender(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    // First value: briefly take the shared lock, then release before channel wait.
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1);

    // Second value: only after receiver took the first (channel capacity 1).
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2);
}

fn receiver(ch: Arc<Channel>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_recv", "ch"); let a = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); let b = ch.recv();
    {
        let _guard = m.lock().unwrap();
    }
    // Ensure both values were received in order.
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}

fn main() { cir_trace::init();
    let ch = Arc::new(Channel::new());
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let ch_s = Arc::clone(&ch);
    let m_s = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender", move || sender(ch_s, m_s));

    let ch_r = Arc::clone(&ch);
    let m_r = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(ch_r, m_r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
