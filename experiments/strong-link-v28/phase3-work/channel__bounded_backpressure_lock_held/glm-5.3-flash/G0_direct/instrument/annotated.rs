mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared lock m, used occasionally by both roles.
    let m = Arc::new(Mutex::new_named("m_mutex0#177", ()));

    // Channel ch with capacity 1: it can hold at most one value.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role: passes two values in order (1, then 2).
    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#410", move || {
        for v in 1..=2 {
            // Take the shared lock only briefly, and release it
            // BEFORE waiting on the channel (R5).
            {
                let _guard = m_sender.lock().unwrap();
                // brief critical section; no channel wait while holding m
            }
            // If the channel is full, this waits until the receiver
            // has taken the previous value (R4, R7).
            cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
        }
        // Sender finishes after passing both values (R3).
    });

    // Receiver role: takes two values.
    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1063", move || {
        for _ in 0..2 {
            // Take the shared lock only briefly, and release it
            // BEFORE waiting on the channel (R5).
            {
                let _guard = m_receiver.lock().unwrap();
                // brief critical section; no channel wait while holding m
            }
            // If the channel is empty, this waits until the sender
            // has put a value (R4).
            cir_trace::record("channel_recv", "rx"); let _v = rx.recv().unwrap();
        }
        // Receiver finishes after taking both values (R3).
    });

    // Both roles run at the same time and both must finish (R1, R6).
    sender.join().unwrap();
    receiver.join().unwrap();

    // Exactly one line of output, then exit (R8).
    println!("DONE done=1");
 cir_trace::finish();}
