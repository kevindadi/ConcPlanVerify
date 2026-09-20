use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared mutex + condvar used by both sender and receiver.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // Rendezvous channel: sender and receiver must meet.
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        // Sender takes the lock, signals readiness, then releases it
        // before blocking on the channel send.
        {
            let (lock, cvar) = &*shared_s;
            let mut ready = lock.lock().unwrap();
            *ready = true;
            cvar.notify_all();
        }
        // Send after releasing the lock so the receiver can proceed.
        tx1.send(1).unwrap();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Receiver waits for the sender's readiness signal while holding
        // the lock, then releases it before blocking on the channel recv.
        {
            let (lock, cvar) = &*shared_r;
            let mut ready = lock.lock().unwrap();
            while !*ready {
                ready = cvar.wait(ready).unwrap();
            }
        }
        // Receive after releasing the lock, so the sender is never blocked
        // on the lock while the receiver waits on the channel.
        let _ = rx1.recv().unwrap();
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
