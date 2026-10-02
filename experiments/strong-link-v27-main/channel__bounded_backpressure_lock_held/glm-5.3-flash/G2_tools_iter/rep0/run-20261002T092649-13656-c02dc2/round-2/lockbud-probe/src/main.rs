//! Sender/receiver over a capacity-1 channel (`ch`) with one shared lock (`m`).
//!
//! Design notes (mapping to requirements):
//! - R1: `main` spawns the `sender` and `receiver` threads; they run concurrently.
//! - R2: `ch` is a `sync_channel(1)` (holds at most one value); `m` is the single
//!   shared `Mutex` both roles use for their brief critical sections.
//! - R3: sender passes 1 then 2; receiver takes two values; both threads then end.
//! - R4/R7: `sync_channel(1)` makes `send` block while the channel is full, so the
//!   sender cannot pass its second value before the receiver has taken the first,
//!   and `recv` blocks while the channel is empty.
//! - R5/R6: both roles follow the SAME resource order, `ch` -> `m`: each role
//!   completes its blocking channel operation FIRST and only then acquires `m`
//!   for a brief critical section. No role ever waits on `ch` while holding `m`,
//!   and there is no lock-order inversion between the two threads, so every
//!   schedule and interleaving terminates (Lockbud-clean).
//! - R8: after both threads are joined, exactly one line `DONE done=1` is printed.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // m: the one shared lock (protects the delivery log).
    // ch: the channel, capacity 1.
    let m: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = sync_channel::<i32>(1);

    // ---- sender role ----
    let sender_m = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for v in 1..=2 {
            // Blocks while ch is full. No lock is held here (R4, R5, R7).
            tx.send(v).unwrap();
            {
                // Brief critical section under m, taken only AFTER the channel
                // operation completed (uniform ch -> m order, R5/R6).
                let mut log = sender_m.lock().unwrap();
                log.push(-v); // record "sent v"
            }
        }
    });

    // ---- receiver role ----
    let receiver_m = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // Blocks while ch is empty. No lock is held here (R4, R5).
            let v = rx.recv().unwrap();
            {
                // Brief critical section under m, taken only AFTER the channel
                // operation completed (uniform ch -> m order, R5/R6).
                let mut log = receiver_m.lock().unwrap();
                log.push(v); // record "took v"
            }
        }
    });

    // R1/R3: wait for both roles to finish.
    sender.join().unwrap();
    receiver.join().unwrap();

    // R8: verify both values were delivered in order, then print the line.
    let done = {
        let log = m.lock().unwrap();
        let received: Vec<i32> = log.iter().copied().filter(|v| *v > 0).collect();
        if received == [1, 2] { 1 } else { 0 }
    };
    println!("DONE done={done}");
}
