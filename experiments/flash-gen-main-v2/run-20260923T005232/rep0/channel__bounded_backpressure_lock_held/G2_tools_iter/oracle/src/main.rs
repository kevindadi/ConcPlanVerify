mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Channel {
    slot: Option<i32>,
    closed: bool,
}

struct Shared {
    m: Mutex<()>,
    ch: Mutex<Channel>,
    not_full: Condvar,
    not_empty: Condvar,
}

fn sender(shared: Arc<Shared>) {
    for value in [1, 2] {
        // Wait until there is room in the channel, without holding the shared lock m.
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_some() {
            ch = shared.not_full.wait(ch).unwrap();
        }
        ch.slot = Some(value);
        shared.not_empty.notify_one();
        drop(ch);

        // Occasionally need the shared lock m, but never while waiting on the channel.
        {
            let _guard = shared.m.lock().unwrap();
        }
    }
}

fn receiver(shared: Arc<Shared>) {
    let mut done = 0;
    for _ in 0..2 {
        // Wait until there is a value in the channel, without holding the shared lock m.
        let mut ch = shared.ch.lock().unwrap();
        while ch.slot.is_none() {
            ch = shared.not_empty.wait(ch).unwrap();
        }
        let _value = ch.slot.take();
        shared.not_full.notify_one();
        drop(ch);

        // Occasionally need the shared lock m, but never while waiting on the channel.
        {
            let _guard = shared.m.lock().unwrap();
        }

        done += 1;
    }

    println!("DONE done={}", done);
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch: Mutex::new_named("shared_mutex1", Channel {
            slot: None,
            closed: false,
        }),
        not_full: Condvar::new_named("shared_condvar0"),
        not_empty: Condvar::new_named("shared_condvar1"),
    });

    let s = Arc::clone(&shared);
    let r = Arc::clone(&shared);

    let sender_handle = cir_trace::spawn("sender_handle", move || sender(s));
    let receiver_handle = cir_trace::spawn("receiver_handle", move || receiver(r));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();
 cir_trace::finish();}
