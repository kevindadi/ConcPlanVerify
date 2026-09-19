use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared mutex + condvar that both sender and receiver occasionally need.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // Rendezvous channel: sender and receiver must meet.
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        // Sender takes the shared lock briefly, then releases it before
        // blocking on the channel send, so the receiver can acquire it.
        {
            let (lock, cvar) = &*shared_s;
            let mut flag = lock.lock().unwrap();
            *flag = true;
            cvar.notify_all();
        }

        // Send to receiver (may block until receiver receives).
        tx1.send(1).unwrap();

        // Wait for receiver's acknowledgement.
        let _ = rx2.recv().unwrap();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Receiver takes the shared lock briefly, then releases it before
        // blocking on the channel receive, so the sender can acquire it.
        {
            let (lock, cvar) = &*shared_r;
            let mut flag = lock.lock().unwrap();
            while !*flag {
                flag = cvar.wait(flag).unwrap();
            }
        }

        // Receive from sender (may block until sender sends).
        let _ = rx1.recv().unwrap();

        // Acknowledge back to sender.
        tx2.send(1).unwrap();
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
