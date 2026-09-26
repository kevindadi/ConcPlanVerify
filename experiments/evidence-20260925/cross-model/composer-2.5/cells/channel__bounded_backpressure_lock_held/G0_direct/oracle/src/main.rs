mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Mutex<Option<u32>>,
    not_full: Condvar,
    not_empty: Condvar,
}

impl Channel {
    fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            not_full: Condvar::new(),
            not_empty: Condvar::new(),
        }
    }

    fn send(&self, value: u32) {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_some() {
            guard = self.not_full.wait(guard).unwrap();
        }
        *guard = Some(value);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> u32 {
        let mut guard = self.slot.lock().unwrap();
        while guard.is_none() {
            guard = self.not_empty.wait(guard).unwrap();
        }
        let value = guard.take().unwrap();
        self.not_full.notify_one();
        value
    }
}

fn sender(m: Arc<Mutex<u32>>, ch: Arc<Channel>, slot: Arc<Semaphore>) {
    {
        let _permit = slot.acquire();
        let mut guard = m.lock().unwrap();
        *guard = *guard;
        drop(guard);
    }

    cir_trace::record("channel_send", "ch"); ch.send(1);

    {
        let _permit = slot.acquire();
        let _guard = m.lock().unwrap();
    }

    cir_trace::record("channel_send", "ch"); ch.send(2);
}

fn receiver(m: Arc<Mutex<u32>>, ch: Arc<Channel>, slot: Arc<Semaphore>) {
    {
        let _permit = slot.acquire();
        let _guard = m.lock().unwrap();
    }

    cir_trace::record("channel_recv", "ch"); let _first = ch.recv();

    {
        let _permit = slot.acquire();
        let mut guard = m.lock().unwrap();
        *guard = 1;
        drop(guard);
    }

    cir_trace::record("channel_recv", "ch"); let _second = ch.recv();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u32));
    let ch = Arc::new(Channel::new());
    let slot = Semaphore::new_named("slot_semaphore0", 1);

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);
    let ch_receiver = Arc::clone(&ch);
    let slot_sender = Arc::clone(&slot);
    let slot_receiver = Arc::clone(&slot);

    let sender_handle = cir_trace::spawn("sender", move || {
        sender(m_sender, ch_sender, slot_sender);
    });
    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(m_receiver, ch_receiver, slot_receiver);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
