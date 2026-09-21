use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    // Rendezvous channel: send blocks until the receiver picks it up.
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    // Shared mutex used by both threads.
    let shared = Arc::new(Mutex::new(0i32));

    let s_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for i in 1..=10 {
            {
                // Sender takes the shared mutex, does its work, and
                // explicitly releases it BEFORE blocking on the channel,
                // so it never blocks on send while holding the lock.
                let mut v = s_shared.lock().unwrap();
                *v += i;
                drop(v);
            }
            // Lock is guaranteed released here.
            tx.send(i).unwrap();
        }
        // tx dropped here: receiver's recv loop will end.
    });

    let r_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        let mut total = 0;
        // The receiver blocks on the channel WITHOUT holding the mutex.
        // It only acquires the lock after a value has arrived, and the
        // guard is explicitly dropped before the next blocking recv.
        loop {
            match rx.recv() {
                Ok(i) => {
                    total += i;
                    let mut v = r_shared.lock().unwrap();
                    *v += 1;
                    drop(v);
                    // Lock is guaranteed released before looping back to recv.
                }
                Err(_) => break,
            }
        }
        total
    });

    sender.join().unwrap();
    let total = receiver.join().unwrap();

    let v = shared.lock().unwrap();
    // Sender added 1..=10 (55); receiver incremented once per message (10).
    assert_eq!(*v, 65);
    assert_eq!(total, 55);
    drop(v);

    println!("DONE done=1");
}
