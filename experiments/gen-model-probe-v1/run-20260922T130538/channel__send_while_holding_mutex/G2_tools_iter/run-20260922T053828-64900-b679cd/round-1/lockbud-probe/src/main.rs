use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // One shared lock used by both roles.
    let shared = Arc::new(Mutex::new(0u32));

    // Rendezvous channel: sender and receiver must meet (capacity 0).
    let (tx, rx) = sync_channel::<u32>(0);

    let sender_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        // Use the shared lock, then release it BEFORE waiting on the channel.
        {
            let mut guard = sender_shared.lock().unwrap();
            *guard += 1;
        }
        // No lock is held while blocking on the rendezvous send.
        tx.send(1u32).unwrap();
    });

    let receiver_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        // No lock is held while blocking on the rendezvous receive.
        let value = rx.recv().unwrap();
        // Use the shared lock only after the channel exchange completed.
        {
            let mut guard = receiver_shared.lock().unwrap();
            *guard += value;
        }
        value
    });

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
}
