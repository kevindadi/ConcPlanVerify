use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        // Sender needs the shared mutex.
        {
            let (lock, cvar) = &*shared_s;
            let mut ready = lock.lock().unwrap();
            *ready = true;
            cvar.notify_all();
        }

        // Send on the channel.
        tx1.send(1).unwrap();

        // Wait for the receiver to acknowledge completion.
        let _ = rx2.recv();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Wait until the sender has released the shared mutex before
        // blocking on the channel, so the receiver never blocks on the
        // channel while holding the lock the sender needs.
        {
            let (lock, cvar) = &*shared_r;
            let mut ready = lock.lock().unwrap();
            while !*ready {
                ready = cvar.wait(ready).unwrap();
            }
        }

        // Receive from the channel without holding the shared lock.
        let _ = rx1.recv();

        // Acknowledge completion to the sender.
        let _ = tx2.send(1);
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
