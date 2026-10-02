mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

// State protected by the shared lock m.
#[derive(Default)]
struct State {
    touches: u32, // evidence both roles use the lock
    done: bool,   // set by receiver after taking both values
}

fn main() { cir_trace::init();
    // ch: a channel that can hold at most one value.
    let (tx, rx) = sync_channel::<i32>(1);
    // m: the one shared lock.
    let m = Arc::new(Mutex::new_named("m_mutex0#444", State::default()));

    // ---- sender role ----
    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#550", move || {
        for v in 1..=2 {
            // Take the lock only briefly, and ALWAYS release it before
            // waiting on the channel (R5: no hold-and-wait on m + ch).
            {
                let mut s = m_sender.lock().unwrap();
                s.touches += 1;
            } // lock released here
            // Blocks when the channel is full (R4). Since capacity is 1,
            // the second send cannot proceed until the receiver has taken
            // the first value (R7).
            cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
        }
    });

    // ---- receiver role ----
    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1204", move || {
        let mut got = 0;
        while got < 2 {
            // Same discipline: lock is released before any channel wait.
            {
                let mut s = m_receiver.lock().unwrap();
                s.touches += 1;
            } // lock released here
            // Blocks when the channel is empty (R4).
            cir_trace::record("channel_recv", "rx"); if rx.recv().is_ok() {
                got += 1;
            }
        }
        let mut s = m_receiver.lock().unwrap();
        s.done = true;
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    let done = {
        let s = m.lock().unwrap();
        s.done
    };

    // R8: print exactly this line, then exit.
    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}
