use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources:
    //   m  — the one shared lock both roles occasionally need (R2)
    //   ch — channel with capacity 1 (R2, R7)
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);

    // ---- sender role ----
    let m_for_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for i in 1..=2 {
            {
                // Brief critical section: lock is acquired and released
                // BEFORE any channel operation that can block (R5).
                let _guard = m_for_sender.lock().unwrap();
            }
            // Blocks while ch is full (R4). Capacity 1 guarantees the
            // second send cannot complete until the first value is
            // taken by the receiver (R7). Not holding m here (R5).
            ch_tx.send(i).unwrap();
        }
    });

    // ---- receiver role ----
    let m_for_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        let mut taken = 0;
        for _ in 0..2 {
            // Blocks while ch is empty (R4). Not holding m here (R5).
            let _v = ch_rx.recv().unwrap();
            {
                // Brief critical section after the receive; released
                // before the next blocking recv (R5).
                let _guard = m_for_receiver.lock().unwrap();
            }
            taken += 1;
        }
        taken
    });

    // Main task waits for both roles to finish (R1, R3, R6).
    sender.join().unwrap();
    let received = receiver.join().unwrap();

    let done = if received == 2 { 1 } else { 0 };
    println!("DONE done={}", done); // exactly `DONE done=1` (R8)
}
