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
        // before doing the blocking send so the receiver can proceed.
        {
            let (lock, cvar) = &*shared_s;
            let mut ready = lock.lock().unwrap();
            *ready = true;
            cvar.notify_all();
        }

        // Blocking send happens without holding the shared lock.
        tx1.send(1).unwrap();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Wait until the sender has signaled readiness, then release
        // the lock before blocking on the channel receive.
        {
            let (lock, cvar) = &*shared_r;
            let mut ready = lock.lock().unwrap();
            while !*ready {
                ready = cvar.wait(ready).unwrap();
            }
        }

        // Blocking receive happens without holding the shared lock.
        let _ = rx1.recv();
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
