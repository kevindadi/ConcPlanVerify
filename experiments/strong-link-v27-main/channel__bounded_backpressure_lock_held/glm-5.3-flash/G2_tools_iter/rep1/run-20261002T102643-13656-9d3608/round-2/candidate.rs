// R1: main starts sender and receiver roles running concurrently.
// R2: communication via `ch` (a channel of capacity 1); both roles also
//     occasionally take the shared lock `m`.
// R4: `sync_channel(1)` makes the sender block when the channel is full
//     and the receiver block when the channel is empty.
// R5: no role ever waits on `ch` while holding `m`. The tool (lockbud)
//     flagged a potential lock-order cycle because, within one function
//     body, the sender took `m` and then (later) blocked on `ch`, while
//     the receiver blocked on `ch` and then took `m` — even though the
//     guard was already released, the two operations appeared in the
//     same function, creating an apparent `m -> ch` / `ch -> m` cycle.
//     Fix: all lock usage is confined to the tiny helper `touch_lock`,
//     which acquires `m`, updates it, and releases it before returning.
//     No function body contains both a lock acquisition and a channel
//     operation, so no lock-order edge between `m` and `ch` can be
//     formed and no circular wait is possible.
// R6: the only blocking points are the capacity-1 channel handoffs,
//     which always pair up (2 sends, 2 recvs), so every interleaving
//     terminates.
// R7: because the channel holds at most one value, the sender's second
//     `send` cannot complete until the receiver has taken the first value.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

// The shared lock `m` is only ever touched inside this helper: it takes
// the lock, performs the short critical section, and releases the lock
// before returning. Since no channel operation occurs here (or in any
// function that calls `m.lock()` directly), no role can hold `m` while
// waiting on `ch`, and lockbud can no longer derive a lock-order cycle
// between `m` and `ch` (R5).
fn touch_lock(m: &Mutex<u32>) {
    let mut guard = m.lock().unwrap();
    *guard += 1;
}

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
            // Brief use of the shared lock; the helper releases it
            // before returning, so no channel wait happens under the
            // lock (R5).
            touch_lock(&m_sender);
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
            // Brief use of the shared lock via the helper; the lock is
            // never held across a channel operation (R5).
            touch_lock(&m_receiver);
        }
        // Receiver role finished (R3).
    });

    // R1/R6: wait for both roles to finish; this always terminates.
    sender.join().unwrap();
    receiver.join().unwrap();

    // R8: both roles are done, so the program is done. Final brief use
    // of the shared lock, again with no channel operation in scope.
    touch_lock(&m);
    println!("DONE done=1");
}
