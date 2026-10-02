use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    // ch: channel holding at most one value (R2, R4, R7)
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    // m: shared lock both roles occasionally need (R2)
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    let m_tx = m.clone();

    let sender = thread::spawn(move || {
        for v in 1..=2i32 {
            // Acquire lock only while NOT blocked on the channel (R5):
            // we take the lock, record nothing blocking, release before send-wait.
            {
                let _g = m_tx.lock().unwrap();
                // short critical section; channel not waited on here
            }
            tx.send(v).unwrap(); // waits while channel full (R4); lock released above
        }
    });

    let m_rx = m.clone();

    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            let v = rx.recv().unwrap(); // waits while channel empty (R4); no lock held
            {
                let _g = m_rx.lock().unwrap();
                let _ = v; // brief use of shared lock (R2), never blocking on channel inside (R5)
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    // R3: both roles finished; R6: termination guaranteed by capacity-1 handoff;
    // R8: exact output
    println!("DONE done=1");
}
