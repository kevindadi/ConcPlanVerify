use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

// Shared state protected by the single lock `m` (R2).
struct State {
    done: bool,
    log: Vec<&'static str>,
}

fn main() {
    // ch: channel with capacity 1 (R2, R4, R7).
    let (tx, rx) = sync_channel::<i32>(1);
    // m: the one shared lock (R2).
    let m = Arc::new(Mutex::new(State {
        done: false,
        log: Vec::new(),
    }));

    // ---- sender role (R1) ----
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        // Blocking send happens WITHOUT holding m (R5).
        // Buffer is empty, so this completes; buffer is now full.
        tx.send(1).expect("receiver alive");

        // Brief critical section; lock released at end of block.
        {
            let mut s = m_sender.lock().expect("m not poisoned");
            s.log.push("sent 1");
        } // m released BEFORE the next blocking send (R5).

        // Channel is full until the receiver takes value 1,
        // so this send waits — enforcing R7. Not holding m (R5).
        tx.send(2).expect("receiver alive");

        {
            let mut s = m_sender.lock().expect("m not poisoned");
            s.log.push("sent 2");
        }
        // Sender finishes (R3).
    });

    // ---- receiver role (R1) ----
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        // Blocking recv happens WITHOUT holding m (R5).
        // Waits if channel is empty (R4).
        let first = rx.recv().expect("sender alive");

        {
            let mut s = m_receiver.lock().expect("m not poisoned");
            s.log.push("got 1");
        } // m released BEFORE the next blocking recv (R5).

        // Taking the first value is what unblocks the sender's
        // second send; then this recv finds value 2 (R3, R7).
        let second = rx.recv().expect("sender alive");

        {
            let mut s = m_receiver.lock().expect("m not poisoned");
            s.log.push("got 2");
            s.done = true;
        }
        let _ = (first, second);
        // Receiver finishes (R3).
    });

    // Main waits for both roles (R1, R3, R6).
    sender.join().expect("sender thread panicked");
    receiver.join().expect("receiver thread panicked");

    // Receiver has set done = true under m; read it under m (R8).
    let s = m.lock().expect("m not poisoned");
    assert!(s.done, "receiver must have finished");
    println!("DONE done={}", s.done as u8);
}
