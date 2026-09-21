mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        chan: Mutex::new_named("shared_mutex0", Channel {
            slot: None,
            closed: false,
        }),
        not_empty: Condvar::new_named("shared_condvar0"),
        not_full: Condvar::new_named("shared_condvar1"),
        lock: Mutex::new_named("shared_mutex1", ()),
    });

    let sender_shared = Arc::clone(&shared);
    let sender = cir_trace::spawn("sender", move || {
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
    let receiver = cir_trace::spawn("receiver", move || {
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
 cir_trace::finish();}
