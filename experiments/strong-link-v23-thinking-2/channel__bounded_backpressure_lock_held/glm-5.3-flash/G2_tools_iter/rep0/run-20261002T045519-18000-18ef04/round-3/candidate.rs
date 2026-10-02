// R1: main starts sender and receiver roles that run concurrently.
// R2: communication via a capacity-1 channel (ch); both roles share the lock (m).
// R4: sync_channel(1) makes send block while the channel is full and recv block
//     while the channel is empty.
// R5: neither role ever holds m while blocked on ch: the sender drops its lock
//     guard before sending, and the receiver receives before taking the lock.
// R6: since m is never held across a channel wait, no interleaving can deadlock;
//     both roles always finish.
// R7: with capacity 1, the sender's second send cannot complete until the
//     receiver has taken the first value.
//
// Lockbud fix: the receiver previously used two adjacent critical sections
// (accumulate in the loop, then set done=1 after the loop). That pair can be
// interleaved by other threads, which Lockbud reports as an atomicity
// violation. The completion flag is now written inside the same critical
// section as the final accumulation, so each received value is handled in
// exactly one critical section.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // m: shared lock protecting `done` (used occasionally by both roles).
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    // ch: channel that holds at most one value.
    let (tx, rx) = sync_channel::<i32>(1);

    // ---- sender role ----
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for i in 1..=2 {
            // Occasional use of the shared lock, released before any send.
            let value = {
                let guard = m_sender.lock().unwrap();
                i * guard.signum().max(1) // derive value under the lock
            }; // lock guard dropped here (R5)

            // Blocks while the channel is full (R4); second send waits until
            // the receiver has taken the first value (R7).
            tx.send(value).expect("receiver alive");
        }
        // Sender role finished (R3).
    });

    // ---- receiver role ----
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        for i in 0..2 {
            // Blocks while the channel is empty (R4); lock is NOT held here (R5).
            let value = rx.recv().expect("sender alive");

            // Single critical section per value: accumulate, and on the last
            // value also mark completion. No second adjacent lock acquisition,
            // so no atomicity-violation window for Lockbud to flag.
            let mut guard = m_receiver.lock().unwrap();
            *guard += value;
            if i == 1 {
                *guard = 1;
            }
        } // lock guard dropped here

        // Receiver role finished (R3).
    });

    // Wait for both roles so every schedule terminates before printing (R6).
    sender.join().expect("sender panicked");
    receiver.join().expect("receiver panicked");

    // R8: print exactly `DONE done=1` and exit.
    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
