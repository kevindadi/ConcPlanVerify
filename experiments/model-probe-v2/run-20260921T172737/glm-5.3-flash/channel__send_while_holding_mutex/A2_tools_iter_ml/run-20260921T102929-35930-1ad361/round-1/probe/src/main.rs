use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);
    let shared = Arc::new((Mutex::new(0i32), Condvar::new()));

    let s_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        // Sender briefly takes the shared mutex, but does NOT hold it
        // while blocking on the channel send.
        {
            let mut guard = s_shared.0.lock().unwrap();
            *guard += 1;
        }
        tx1.send(1).unwrap();
        let _ = tx2; // keep tx2 alive; receiver drops rx2
    });

    let r_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        // Receiver must not block on the channel while holding the lock
        // the sender needs: take the lock only after receiving.
        let v = rx1.recv().unwrap();
        {
            let mut guard = r_shared.0.lock().unwrap();
            *guard += v;
        }
        let _ = rx2; // drop rx2 so tx2's implicit drop never blocks
    });

    sender.join().unwrap();
    receiver.join().unwrap();
    println!("DONE done=1");
}
