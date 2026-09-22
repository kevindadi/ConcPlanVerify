use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// A bounded channel with capacity one.
///
/// The channel has its own internal lock (`slot`) plus two condition
/// variables. This lock is distinct from the "shared lock" that both
/// roles occasionally need, so neither role ever waits on the channel
/// while holding the shared lock.
struct Channel {
    slot: Mutex<Option<u64>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Channel {
    fn new() -> Self {
        Channel {
            slot: Mutex::new(None),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    /// Blocks while the channel is full (capacity one), then deposits `v`.
    fn send(&self, v: u64) {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_some() {
            // Wait releases `slot` (the channel lock) while blocked.
            slot = self.not_full.wait(slot).unwrap();
        }
        *slot = Some(v);
        self.not_empty.notify_one();
    }

    /// Blocks while the channel is empty, then takes the value.
    fn recv(&self) -> u64 {
        let mut slot = self.slot.lock().unwrap();
        while slot.is_none() {
            // Wait releases `slot` (the channel lock) while blocked.
            slot = self.not_empty.wait(slot).unwrap();
        }
        let v = slot.take().unwrap();
        self.not_full.notify_one();
        v
    }
}

fn main() {
    let channel = Arc::new(Channel::new());
    // The one shared lock both roles occasionally need.
    let shared = Arc::new(Mutex::new(0u64));

    // Sender role: passes two values in order.
    let send_chan = Arc::clone(&channel);
    let send_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for v in [1u64, 2u64] {
            // Occasionally use the shared lock, but release it *before*
            // any channel operation that might block.
            {
                let mut acc = send_shared.lock().unwrap();
                *acc += 1;
            } // shared lock released here

            // May wait for the receiver to drain the full channel, but
            // never while holding the shared lock.
            send_chan.send(v);
        }
    });

    // Receiver role: takes two values.
    let recv_chan = Arc::clone(&channel);
    let recv_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // May wait for the sender to fill the empty channel, but
            // never while holding the shared lock.
            let v = recv_chan.recv();

            // Occasionally use the shared lock after the channel
            // operation has completed.
            {
                let mut acc = recv_shared.lock().unwrap();
                *acc += v;
            } // shared lock released here
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
