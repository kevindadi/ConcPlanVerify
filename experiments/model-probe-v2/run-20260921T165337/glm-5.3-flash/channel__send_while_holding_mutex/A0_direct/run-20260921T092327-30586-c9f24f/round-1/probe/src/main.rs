use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Rendezvous channel: send blocks until the receiver picks it up.
    let (tx, rx) = sync_channel::<i32>(0);

    // Shared mutex protecting shared state that both sides occasionally need.
    let shared = Arc::new(Mutex::new(0i64));

    let shared_sender = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for i in 1..=5 {
            // Use the shared mutex, but never while blocking on the channel:
            // acquire, update, release, then send.
            {
                let mut guard = shared_sender.lock().unwrap();
                *guard += i as i64;
            }
            tx.send(i).unwrap();
        }
        // Sentinel value tells the receiver to stop.
        tx.send(0).unwrap();
    });

    let shared_receiver = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        loop {
            // Block on the channel WITHOUT holding the lock, so the sender
            // can always make progress on the mutex.
            let msg = rx.recv().unwrap();
            if msg == 0 {
                break;
            }
            // Only after the message is fully received do we take the lock.
            let mut guard = shared_receiver.lock().unwrap();
            *guard += 1;
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    let final_value = *shared.lock().unwrap();
    // The sender added 1+2+3+4+5 = 15; the receiver incremented once per
    // message (5 times). done=1 marks successful completion.
    let done = if final_value == 20 { 1 } else { 0 };
    println!("DONE done={}", done);
}
