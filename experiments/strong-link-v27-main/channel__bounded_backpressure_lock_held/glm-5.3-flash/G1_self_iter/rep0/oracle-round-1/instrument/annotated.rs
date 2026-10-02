mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // m: shared lock guarding the done flag (both roles use it).
    // ch: channel with capacity 1 (at most one value in flight).
    let m = Arc::new(Mutex::new_named("m_mutex0#254", 0u32));
    let (ch_tx, ch_rx) = sync_channel::<u32>(1);

    // ---- sender role ----
    let m_for_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#401", move || {
        // Use the shared lock, but release it BEFORE waiting on the
        // channel (R5). The capacity-1 channel blocks this second send
        // until the receiver has taken the first value (R4, R7).
        for value in [1u32, 2] {
            {
                let mut guard = m_for_sender.lock().unwrap();
                *guard = guard.wrapping_add(0); // brief, non-panicking critical section
            } // guard dropped here: lock is NOT held during ch.send
            cir_trace::record("channel_send", "ch_tx"); ch_tx.send(value).unwrap();
        }
    });

    // ---- receiver role ----
    let m_for_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1049", move || {
        // Take two values in order (R3). recv blocks while the channel
        // is empty (R4) and is called WITHOUT holding m (R5).
        cir_trace::record("channel_recv", "ch_rx"); let _first = ch_rx.recv().unwrap();
        cir_trace::record("channel_recv", "ch_rx"); let _second = ch_rx.recv().unwrap();

        // Publish completion under the shared lock, after all channel
        // waits are finished.
        let mut guard = m_for_receiver.lock().unwrap();
        *guard = 1;
    });

    // R1/R6: wait for both roles to finish before reporting.
    sender.join().unwrap();
    receiver.join().unwrap();

    let done = *m.lock().unwrap();
    // R8: exactly one line, then exit.
    println!("DONE done={}", done);
 cir_trace::finish();}
