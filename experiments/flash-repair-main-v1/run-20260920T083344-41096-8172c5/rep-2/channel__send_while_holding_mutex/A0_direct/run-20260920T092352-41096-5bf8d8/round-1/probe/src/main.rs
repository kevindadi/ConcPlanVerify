use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new((Mutex::new(0i32), Condvar::new()));

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        // Sender needs the shared mutex.
        {
            let (lock, cvar) = &*shared_s;
            let mut guard = lock.lock().unwrap();
            *guard += 1;
            cvar.notify_all();
        }
        // Send on the channel without holding the lock.
        tx1.send(1).unwrap();
        // Wait for the receiver's acknowledgement.
        let _ = rx2.recv();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        // Receive from the channel without holding the lock.
        let _ = rx1.recv();
        // Receiver needs the shared mutex.
        {
            let (lock, cvar) = &*shared_r;
            let mut guard = lock.lock().unwrap();
            while *guard == 0 {
                guard = cvar.wait(guard).unwrap();
            }
        }
        // Acknowledge completion so the sender can finish.
        tx2.send(1).unwrap();
    });

    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
