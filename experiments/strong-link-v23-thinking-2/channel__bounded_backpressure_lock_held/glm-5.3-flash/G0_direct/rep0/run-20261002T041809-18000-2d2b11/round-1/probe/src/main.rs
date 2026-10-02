// Entities: roles = sender, receiver; shared resources = m (lock), ch (channel).
// ch is a bounded channel with capacity 1 (holds at most one value).
// m is a shared lock both roles occasionally need.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared lock m.
    let m = Arc::new(Mutex::new(()));

    // Channel ch with capacity 1: send blocks while full, recv blocks while empty.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role: passes two values in order.
    let m_for_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for value in 1..=2 {
            // Occasionally take the shared lock m, but never while blocking on ch.
            {
                let _guard = m_for_sender.lock().unwrap();
                // (brief critical section: prepare the value)
            } // lock released before touching the channel

            // With capacity 1, the second send cannot complete until the
            // receiver has taken the first value (R7). If ch is full, this
            // waits (R4) — and it does so without holding m (R5).
            tx.send(value).expect("sender: channel closed");
        }
        // Sender finishes after passing both values (R3).
    });

    // Receiver role: takes two values.
    let m_for_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // If ch is empty, this waits (R4) — without holding m (R5).
            let _value = rx.recv().expect("receiver: channel closed");

            // Occasionally take the shared lock m after the value is taken.
            let _guard = m_for_receiver.lock().unwrap();
            // (brief critical section: process the value)
        }
        // Receiver finishes after taking both values (R3).
        1 // done flag
    });

    // Main task waits for both roles; every schedule terminates (R6).
    sender.join().unwrap();
    let done = receiver.join().unwrap();

    // Exactly one line of output (R8).
    println!("DONE done={}", done);
}
