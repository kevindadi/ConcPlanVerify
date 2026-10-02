mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: main starts sender and receiver roles running concurrently.
// R2: roles share a bounded channel `ch` (capacity 1) and a mutex `m`.
// R4: sync_channel blocks the sender when full and the receiver when empty.
// R5: the lock `m` is never held while waiting on `ch` (lock scope ends
//     before any send/recv that may block).
// R6: with capacity 1 and no lock held across channel waits, every
//     interleaving terminates.
// R7: capacity 1 means the second send cannot complete until the first
//     value has been received.
// R8: prints exactly `DONE done=1` and exits.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared lock m.
    let m = Arc::new(Mutex::new_named("m_mutex0#730", ()));

    // Channel ch with capacity 1.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role.
    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#892", move || {
        // R3: pass two values in order (1 then 0, so the total is 1).
        for v in [1, 0] {
            {
                // Occasionally need the shared lock — but never while
                // waiting on the channel (R5).
                let _guard = m_sender.lock().unwrap();
            } // lock released here, before the possibly-blocking send

            // R4/R7: blocks while the channel is full, i.e. until the
            // receiver has taken the previous value.
            cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
        }
    });

    // Receiver role.
    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1530", move || {
        let mut done = 0;
        // R3: take two values.
        for _ in 0..2 {
            {
                // Occasionally need the shared lock — never held while
                // waiting on the channel (R5).
                let _guard = m_receiver.lock().unwrap();
            } // lock released here, before the possibly-blocking recv

            // R4: blocks while the channel is empty.
            cir_trace::record("channel_recv", "rx"); let v = rx.recv().unwrap();
            done += v;
        }
        done
    });

    // R6: wait for both roles to finish.
    sender.join().unwrap();
    let done = receiver.join().unwrap();

    // R8: print exactly `DONE done=1`.
    println!("DONE done={}", done);
 cir_trace::finish();}
