use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Channel {
    slot: Option<i32>,
    closed: bool,
}

struct Shared {
    chan: Mutex<Channel>,
    not_empty: Condvar,
    not_full: Condvar,
    lock: Mutex<()>,
}

fn main() {
    let shared = Arc::new(Shared {
        chan: Mutex::new(Channel {
            slot: None,
            closed: false,
        }),
        not_empty: Condvar::new(),
        not_full: Condvar::new(),
        lock: Mutex::new(()),
    });

    let sender_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for value in [1, 2] {
            // Acquire the shared lock occasionally, but never wait on the
            // channel while holding it.
            {
                let _guard = sender_shared.lock.lock().unwrap();
            }

            let mut chan = sender_shared.chan.lock().unwrap();
            while chan.slot.is_some() {
                chan = sender_shared.not_full.wait(chan).unwrap();
            }
            chan.slot = Some(value);
            sender_shared.not_empty.notify_one();
        }
    });

    let receiver_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        let mut done = 0;
        for _ in 0..2 {
            // Acquire the shared lock occasionally, but never wait on the
            // channel while holding it.
            {
                let _guard = receiver_shared.lock.lock().unwrap();
            }

            let mut chan = receiver_shared.chan.lock().unwrap();
            while chan.slot.is_none() {
                chan = receiver_shared.not_empty.wait(chan).unwrap();
            }
            let _value = chan.slot.take();
            done += 1;
            receiver_shared.not_full.notify_one();
        }
        println!("DONE done={}", done);
    });

    sender.join().unwrap();
    receiver.join().unwrap();
}
