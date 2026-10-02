// R1: main starts sender and receiver roles running concurrently.
// R2: communication via `ch` (a channel of capacity 1); both roles also
//     occasionally take the shared lock `m`.
// R4: `sync_channel(1)` makes the sender block when the channel is full
//     and the receiver block when the channel is empty.
// R5: neither role ever holds `m` while waiting on `ch` — `m` is always
//     locked and released in a short critical section that contains no
//     channel operation, so no circular wait (and thus no deadlock) is
//     possible.
// R6: the only blocking points are the capacity-1 channel handoffs,
//     which always pair up (2 sends, 2 recvs), so every interleaving
//     terminates.
// R7: because the channel holds at most one value, the sender's second
//     `send` cannot complete until the receiver has taken the first value.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // ch: channel with capacity 1 (R2, R4, R7)
    let (tx, rx) = sync_channel::<i32>(1);

    // m: shared lock both roles occasionally need (R2)
    let m = Arc::new(Mutex::new(0u32));

    // ---- sender role ----
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        // R3: pass two values in order
        for v in 1..=2 {
            {
                // Brief use of the shared lock; released before any
                // channel wait (R5).
                let mut guard = m_sender.lock().unwrap();
                *guard += 1;
            }
            // Blocks while the channel is full (R4); for the second
            // value this waits until the receiver took the first (R7).
            tx.send(v).expect("receiver alive");
        }
        // Sender role finished (R3).
    });

    // ---- receiver role ----
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        // R3: take two values
        for _ in 0..2 {
            // Blocks while the channel is empty (R4).
            let _v = rx.recv().expect("sender alive");
            {
                // Brief use of the shared lock; no channel wait is
                // performed while holding it (R5).
                let mut guard = m_receiver.lock().unwrap();
                *guard += 1;
            }
        }
        // Receiver role finished (R3).
    });

    // R1/R6: wait for both roles to finish; this always terminates.
    sender.join().unwrap();
    receiver.join().unwrap();

    // R8: both roles are done, so the program is done.
    let finished = {
        let _guard = m.lock().unwrap();
        1
    };
    let _ = finished;
    println!("DONE done=1");
}
