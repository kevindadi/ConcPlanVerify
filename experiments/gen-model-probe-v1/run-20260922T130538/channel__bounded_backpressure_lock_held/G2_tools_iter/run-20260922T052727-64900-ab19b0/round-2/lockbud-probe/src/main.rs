use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Channel that can hold at most one value (R2). A send blocks while
    // the slot is still occupied (R4), so the second value cannot be sent
    // before the first one is taken (R7).
    let (tx, rx) = sync_channel::<i64>(1);

    // The one shared lock both roles occasionally need (R2).
    let shared = Arc::new(Mutex::new(1i64));

    // Sender role (R1).
    let shared_sender = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for value in 1..=2i64 {
            {
                // Take the shared lock briefly and release it *before*
                // touching the channel (R5).
                let mut guard = shared_sender.lock().unwrap();
                *guard += 1;
            }
            // Waits while the channel is full, but never while holding
            // the lock (R4, R5). Sends the two values in order (R3).
            tx.send(value).unwrap();
        }
    });

    // Receiver role (R1).
    let shared_receiver = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // Waits while the channel is empty, without holding the
            // shared lock (R4, R5). Takes the two values (R3).
            let value = rx.recv().unwrap();
            {
                let mut guard = shared_receiver.lock().unwrap();
                *guard -= value - (value - 1); // decrement by 1
            }
        }
    });

    // Both roles run to completion under every interleaving (R6).
    sender.join().unwrap();
    receiver.join().unwrap();

    // Final shared state: 1 + 1 + 1 - 1 - 1 = 1.
    let done = *shared.lock().unwrap();
    println!("DONE done={}", done); // (R8)
}
